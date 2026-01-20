//! Workflow execution.

use std::collections::HashMap;

use fuschia_workflow::{LockedComponent, NodeType};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

use crate::error::RuntimeError;
use crate::runtime::WorkflowRuntime;
use crate::task::TaskResult;

/// Result of a workflow execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowResult {
  /// Unique execution ID.
  pub execution_id: String,
  /// Results of all executed tasks, keyed by node_id.
  pub task_results: HashMap<String, TaskResult>,
}

/// A handle to a workflow execution.
///
/// Call `.wait()` to run the execution and get the result.
pub struct WorkflowExecution<'a> {
  runtime: &'a WorkflowRuntime,
  execution_id: String,
  payload: serde_json::Value,
  cancel: CancellationToken,
}

impl<'a> WorkflowExecution<'a> {
  pub(crate) fn new(
    runtime: &'a WorkflowRuntime,
    execution_id: String,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Self {
    Self {
      runtime,
      execution_id,
      payload,
      cancel,
    }
  }

  /// Wait for the workflow to complete.
  #[instrument(
    name = "workflow_execute",
    skip(self),
    fields(
      workflow_id = %self.runtime.workflow.workflow_id,
      execution_id = %self.execution_id,
    )
  )]
  pub async fn wait(self) -> Result<WorkflowResult, RuntimeError> {
    // Emit workflow_started event
    info!(
      execution_id = %self.execution_id,
      workflow_id = %self.runtime.workflow.workflow_id,
      trigger_payload = %self.payload,
      "workflow_started"
    );

    // Validate the workflow graph
    self.validate_workflow()?;

    // Initialize completed results with trigger node payloads
    let mut completed = self.initialize_triggers()?;

    // Run the execution loop
    let result = self.run_loop(&mut completed).await;

    // Emit completion/failure event
    match &result {
      Ok(_) => {
        info!(
          execution_id = %self.execution_id,
          "workflow_completed"
        );
      }
      Err(e) => {
        error!(
          execution_id = %self.execution_id,
          error = %e,
          "workflow_failed"
        );
      }
    }

    result
  }

  /// Run the main execution loop.
  async fn run_loop(
    &self,
    completed: &mut HashMap<String, TaskResult>,
  ) -> Result<WorkflowResult, RuntimeError> {
    loop {
      if self.cancel.is_cancelled() {
        warn!(execution_id = %self.execution_id, "workflow cancelled");
        return Err(RuntimeError::Cancelled);
      }

      let ready = self.find_ready_nodes(completed);
      if ready.is_empty() {
        break;
      }

      info!(
        execution_id = %self.execution_id,
        ready_nodes = ?ready,
        "executing batch of ready nodes"
      );

      // Create task executions for all ready nodes
      let executions = self.create_task_executions(&ready, completed)?;

      // Execute all tasks concurrently
      let handles: Vec<_> = executions
        .into_iter()
        .map(|exec| {
          let node_id = exec.node_id().to_string();
          tokio::spawn(async move {
            let result = exec.wait().await;
            (node_id, result)
          })
        })
        .collect();

      // Wait for all tasks
      let results = tokio::select! {
          results = futures::future::join_all(handles) => results,
          _ = self.cancel.cancelled() => {
            warn!(execution_id = %self.execution_id, "workflow cancelled during task execution");
            return Err(RuntimeError::Cancelled);
          }
      };

      // Process results
      for result in results {
        let (node_id, task_result) = result.map_err(|e| RuntimeError::InvalidGraph {
          message: format!("task join error: {}", e),
        })?;

        match task_result {
          Ok(task_result) => {
            completed.insert(node_id, task_result);
          }
          Err(e) => {
            return Err(e);
          }
        }
      }
    }

    Ok(WorkflowResult {
      execution_id: self.execution_id.clone(),
      task_results: completed.clone(),
    })
  }

  /// Initialize trigger nodes with the payload.
  fn initialize_triggers(&self) -> Result<HashMap<String, TaskResult>, RuntimeError> {
    let trigger_nodes = self.find_trigger_nodes();
    if trigger_nodes.is_empty() {
      return Err(RuntimeError::InvalidGraph {
        message: "workflow has no trigger nodes".to_string(),
      });
    }

    let mut completed = HashMap::new();
    for trigger_id in trigger_nodes {
      completed.insert(
        trigger_id.clone(),
        TaskResult {
          task_id: uuid::Uuid::new_v4().to_string(),
          node_id: trigger_id,
          input: serde_json::Value::Null,
          resolved_input: serde_json::Value::Null,
          output: self.payload.clone(),
        },
      );
    }
    Ok(completed)
  }

  /// Find all trigger nodes in the workflow.
  fn find_trigger_nodes(&self) -> Vec<String> {
    self
      .runtime
      .workflow
      .nodes
      .iter()
      .filter(|(_, node)| matches!(node.node_type, NodeType::Trigger(_)))
      .map(|(id, _)| id.clone())
      .collect()
  }

  /// Find nodes that are ready to execute (all upstream nodes completed).
  fn find_ready_nodes(&self, completed: &HashMap<String, TaskResult>) -> Vec<String> {
    let graph = self.runtime.workflow.graph();

    self
      .runtime
      .workflow
      .nodes
      .keys()
      .filter(|id| !completed.contains_key(*id))
      .filter(|id| {
        graph
          .upstream(id)
          .iter()
          .all(|up| completed.contains_key(up))
      })
      .cloned()
      .collect()
  }

  /// Create task executions for all ready nodes.
  fn create_task_executions(
    &self,
    ready: &[String],
    completed: &HashMap<String, TaskResult>,
  ) -> Result<Vec<TaskExecutionOwned>, RuntimeError> {
    let graph = self.runtime.workflow.graph();
    let mut executions = Vec::with_capacity(ready.len());

    for node_id in ready {
      let node = self.runtime.get_node(node_id).unwrap().clone();
      let upstream_ids: Vec<String> = graph.upstream(node_id).iter().cloned().collect();
      let is_join = graph.is_join_point(node_id);

      // Gather upstream data (using output from completed tasks)
      let upstream_data: HashMap<String, serde_json::Value> = upstream_ids
        .iter()
        .filter_map(|id| completed.get(id).map(|r| (id.clone(), r.output.clone())))
        .collect();

      // Build input from upstream data
      let input = if is_join {
        serde_json::Value::Object(
          upstream_data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        )
      } else if upstream_data.len() == 1 {
        upstream_data
          .values()
          .next()
          .cloned()
          .unwrap_or(serde_json::Value::Null)
      } else {
        serde_json::Value::Null
      };

      let task_id = uuid::Uuid::new_v4().to_string();

      executions.push(TaskExecutionOwned::new(
        self.runtime.engine.clone(),
        self.runtime.component_cache.clone(),
        self.runtime.config.clone(),
        self.execution_id.clone(),
        task_id,
        node,
        input,
        upstream_data,
        is_join,
        self.cancel.clone(),
      ));
    }

    Ok(executions)
  }

  /// Validate the workflow graph.
  fn validate_workflow(&self) -> Result<(), RuntimeError> {
    let graph = self.runtime.workflow.graph();
    let entry_points = graph.entry_points();

    for entry_id in entry_points {
      let node = self
        .runtime
        .get_node(entry_id)
        .ok_or_else(|| RuntimeError::InvalidGraph {
          message: format!("entry point '{}' not found in nodes", entry_id),
        })?;

      if !matches!(node.node_type, NodeType::Trigger(_)) {
        return Err(RuntimeError::InvalidGraph {
          message: format!(
            "node '{}' has no incoming edges but is not a trigger (orphan node)",
            entry_id
          ),
        });
      }
    }

    Ok(())
  }
}

// Helper methods for TaskExecutionOwned
impl TaskExecutionOwned {
  fn node_id(&self) -> &str {
    &self.node.node_id
  }
}

/// An owned version of TaskExecution that can be sent across threads.
///
/// This is used when spawning tasks in the workflow execution loop.
pub(crate) struct TaskExecutionOwned {
  engine: std::sync::Arc<wasmtime::Engine>,
  component_cache: crate::cache::ComponentCache,
  config: crate::runtime::RuntimeConfig,
  execution_id: String,
  task_id: String,
  node: fuschia_workflow::Node,
  input: serde_json::Value,
  upstream_data: HashMap<String, serde_json::Value>,
  is_join: bool,
  cancel: CancellationToken,
}

impl TaskExecutionOwned {
  #[allow(clippy::too_many_arguments)]
  fn new(
    engine: std::sync::Arc<wasmtime::Engine>,
    component_cache: crate::cache::ComponentCache,
    config: crate::runtime::RuntimeConfig,
    execution_id: String,
    task_id: String,
    node: fuschia_workflow::Node,
    input: serde_json::Value,
    upstream_data: HashMap<String, serde_json::Value>,
    is_join: bool,
    cancel: CancellationToken,
  ) -> Self {
    Self {
      engine,
      component_cache,
      config,
      execution_id,
      task_id,
      node,
      input,
      upstream_data,
      is_join,
      cancel,
    }
  }

  /// Wait for the task to complete.
  #[instrument(
    name = "task_execute",
    skip(self),
    fields(
      execution_id = %self.execution_id,
      task_id = %self.task_id,
      node_id = %self.node.node_id,
    )
  )]
  pub async fn wait(self) -> Result<TaskResult, RuntimeError> {
    use crate::input::{coerce_inputs, extract_schema_types, resolve_inputs};

    if self.cancel.is_cancelled() {
      return Err(RuntimeError::Cancelled);
    }

    // Resolve inputs based on node type
    let resolved_input = match &self.node.node_type {
      NodeType::Component(locked) => {
        let resolved_strings = resolve_inputs(
          &self.node.node_id,
          &self.node.inputs,
          &self.upstream_data,
          self.is_join,
        )
        .map_err(|e| RuntimeError::InputResolution {
          node_id: self.node.node_id.clone(),
          message: e.to_string(),
        })?;

        let schema = extract_schema_types(&locked.input_schema);
        let resolved =
          coerce_inputs(&self.node.node_id, &resolved_strings, &schema).map_err(|e| {
            RuntimeError::InputResolution {
              node_id: self.node.node_id.clone(),
              message: e.to_string(),
            }
          })?;

        serde_json::to_value(&resolved).unwrap_or(serde_json::Value::Null)
      }
      NodeType::Join { .. } => self.input.clone(),
      _ => serde_json::Value::Null,
    };

    // Emit task_started event (for persistence)
    info!(
      execution_id = %self.execution_id,
      task_id = %self.task_id,
      node_id = %self.node.node_id,
      input = %self.input,
      resolved_input = %resolved_input,
      "task_started"
    );

    let result = match &self.node.node_type {
      NodeType::Component(locked) => self.execute_component(locked, &resolved_input).await,
      NodeType::Join { .. } => Ok(TaskResult {
        task_id: self.task_id.clone(),
        node_id: self.node.node_id.clone(),
        input: self.input.clone(),
        resolved_input: resolved_input.clone(),
        output: self.input.clone(),
      }),
      NodeType::Trigger(_) => Err(RuntimeError::InvalidGraph {
        message: "trigger nodes should not be executed".to_string(),
      }),
      NodeType::Loop(_) => Err(RuntimeError::InvalidGraph {
        message: "loop nodes not yet implemented".to_string(),
      }),
    };

    // Emit completion/failure event
    match &result {
      Ok(task_result) => {
        info!(
          execution_id = %self.execution_id,
          task_id = %self.task_id,
          node_id = %self.node.node_id,
          output = %task_result.output,
          "task_completed"
        );
      }
      Err(e) => {
        error!(
          execution_id = %self.execution_id,
          task_id = %self.task_id,
          node_id = %self.node.node_id,
          error = %e,
          "task_failed"
        );
      }
    }

    result
  }

  /// Execute a component node.
  async fn execute_component(
    &self,
    locked: &LockedComponent,
    resolved_input: &serde_json::Value,
  ) -> Result<TaskResult, RuntimeError> {
    use crate::cache::ComponentKey;
    use fuschia_task_host::{Context, TaskHostState, execute_task};

    // Load component
    let key = ComponentKey::new(&locked.name, &locked.version);
    let sanitized_name = locked.name.replace('/', "--");
    let dir_name = format!("{}--{}", sanitized_name, locked.version);
    let wasm_path = self
      .config
      .component_base_path
      .join(&dir_name)
      .join("component.wasm");

    let component = self
      .component_cache
      .get_or_compile(&self.engine, &key, &wasm_path)
      .map_err(|e| RuntimeError::InvalidGraph {
        message: format!("failed to load component: {}", e),
      })?;

    // Create host state and context
    let state = TaskHostState::new(self.execution_id.clone(), self.node.node_id.clone());
    let ctx = Context {
      execution_id: self.execution_id.clone(),
      node_id: self.node.node_id.clone(),
      task_id: self.task_id.clone(),
    };

    // Serialize resolved input for the component
    let input_data =
      serde_json::to_string(resolved_input).map_err(|e| RuntimeError::InputResolution {
        node_id: self.node.node_id.clone(),
        message: format!("failed to serialize inputs: {}", e),
      })?;

    // Calculate epoch deadline based on timeout
    let epoch_deadline = self.node.timeout_ms.map(|ms| ms / 10);

    // Execute the task
    let result = execute_task(
      &self.engine,
      &component,
      state,
      ctx,
      input_data,
      epoch_deadline,
    )
    .await
    .map_err(|e| RuntimeError::ComponentExecution {
      node_id: self.node.node_id.clone(),
      source: e,
    })?;

    // Parse output data
    let output: serde_json::Value =
      serde_json::from_str(&result.output.data).map_err(|e| RuntimeError::ComponentExecution {
        node_id: self.node.node_id.clone(),
        source: fuschia_host::HostError::component_error(format!("invalid output JSON: {}", e)),
      })?;

    Ok(TaskResult {
      task_id: self.task_id.clone(),
      node_id: self.node.node_id.clone(),
      input: self.input.clone(),
      resolved_input: resolved_input.clone(),
      output,
    })
  }
}
