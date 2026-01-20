//! Workflow executor implementation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use fuschia_task_executor::{TaskExecutor, TaskInput, TaskResult};
use fuschia_workflow::{LockedComponent, NodeType, Workflow};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};
use wasmtime::Engine;
use wasmtime::component::Component;

use crate::cache::{ComponentCache, ComponentKey};
use crate::error::ExecutionError;
use crate::input::{coerce_inputs, extract_schema_types, resolve_inputs};
use crate::result::ExecutionResult;

/// Configuration for the workflow executor.
pub struct ExecutorConfig {
  /// Base path for resolving component wasm files.
  pub component_base_path: PathBuf,
}

/// The workflow executor.
///
/// Handles graph traversal, scheduling, template resolution, and orchestrates
/// task execution via [`TaskExecutor`].
pub struct WorkflowExecutor {
  engine: Arc<Engine>,
  component_cache: ComponentCache,
  config: ExecutorConfig,
}

impl WorkflowExecutor {
  /// Create a new workflow executor.
  pub fn new(config: ExecutorConfig) -> Result<Self, ExecutionError> {
    let mut wasmtime_config = wasmtime::Config::new();
    wasmtime_config.async_support(true);
    wasmtime_config.epoch_interruption(true);

    let engine =
      Arc::new(
        Engine::new(&wasmtime_config).map_err(|e| ExecutionError::InvalidGraph {
          message: format!("failed to create wasmtime engine: {}", e),
        })?,
      );

    Ok(Self {
      engine,
      component_cache: ComponentCache::new(),
      config,
    })
  }

  /// Execute a workflow with the given trigger payload.
  #[instrument(
    name = "workflow_execute",
    skip(self, workflow, payload, cancel),
    fields(
      workflow_id = %workflow.workflow_id,
    )
  )]
  pub async fn execute(
    &self,
    workflow: &Workflow,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<ExecutionResult, ExecutionError> {
    let execution_id = uuid::Uuid::new_v4().to_string();

    info!(
      execution_id = %execution_id,
      workflow_id = %workflow.workflow_id,
      trigger_payload = %payload,
      "workflow_started"
    );

    // Validate the workflow graph
    self.validate_workflow(workflow)?;

    // Initialize completed results with trigger node payloads
    let mut completed = self.initialize_triggers(workflow, payload)?;

    // Execute tasks until no more nodes are ready
    let result = self
      .run_execution_loop(workflow, &mut completed, &execution_id, &cancel)
      .await;

    // Log final status (for persistence)
    match &result {
      Ok(_) => {
        info!(
          execution_id = %execution_id,
          "workflow_completed"
        );
      }
      Err(e) => {
        error!(
          execution_id = %execution_id,
          error = %e,
          "workflow_failed"
        );
      }
    }

    result
  }

  /// Run the main execution loop.
  async fn run_execution_loop(
    &self,
    workflow: &Workflow,
    completed: &mut HashMap<String, TaskResult>,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<ExecutionResult, ExecutionError> {
    loop {
      if cancel.is_cancelled() {
        warn!(execution_id = %execution_id, "workflow cancelled");
        return Err(ExecutionError::Cancelled);
      }

      let ready = self.find_ready_nodes(workflow, completed);
      if ready.is_empty() {
        break;
      }

      info!(
        execution_id = %execution_id,
        ready_nodes = ?ready,
        "executing batch of ready nodes"
      );

      let handles = self.execute_ready_nodes(workflow, &ready, completed, execution_id, cancel)?;

      // Wait for all tasks
      let results = tokio::select! {
          results = futures::future::join_all(handles) => results,
          _ = cancel.cancelled() => {
            warn!(execution_id = %execution_id, "workflow cancelled during task execution");
            return Err(ExecutionError::Cancelled);
          }
      };

      // Process results
      for result in results {
        let join_result = result.map_err(|e| ExecutionError::InvalidGraph {
          message: format!("task join error: {}", e),
        })?;

        match join_result {
          Ok(task_result) => {
            // Emit task_completed event (for persistence)
            info!(
              execution_id = %execution_id,
              task_id = %task_result.task_id,
              node_id = %task_result.node_id,
              output = %task_result.output,
              "task_completed"
            );
            completed.insert(task_result.node_id.clone(), task_result);
          }
          Err((task_id, node_id, e)) => {
            // Emit task_failed event (for persistence)
            error!(
              execution_id = %execution_id,
              task_id = %task_id,
              node_id = %node_id,
              error = %e,
              "task_failed"
            );
            return Err(ExecutionError::TaskExecution { node_id, source: e });
          }
        }
      }
    }

    Ok(ExecutionResult {
      execution_id: execution_id.to_string(),
      task_results: completed.clone(),
    })
  }

  /// Initialize trigger nodes with the payload.
  fn initialize_triggers(
    &self,
    workflow: &Workflow,
    payload: serde_json::Value,
  ) -> Result<HashMap<String, TaskResult>, ExecutionError> {
    let trigger_nodes = self.find_trigger_nodes(workflow);
    if trigger_nodes.is_empty() {
      return Err(ExecutionError::InvalidGraph {
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
          output: payload.clone(),
        },
      );
    }
    Ok(completed)
  }

  /// Find nodes that are ready to execute (all upstream nodes completed).
  fn find_ready_nodes(
    &self,
    workflow: &Workflow,
    completed: &HashMap<String, TaskResult>,
  ) -> Vec<String> {
    let graph = workflow.graph();

    workflow
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

  /// Spawn tasks to execute all ready nodes in parallel.
  fn execute_ready_nodes(
    &self,
    workflow: &Workflow,
    ready: &[String],
    completed: &HashMap<String, TaskResult>,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<
    Vec<
      tokio::task::JoinHandle<
        Result<TaskResult, (String, String, fuschia_task_executor::TaskExecutionError)>,
      >,
    >,
    ExecutionError,
  > {
    let graph = workflow.graph();
    let mut handles = Vec::with_capacity(ready.len());

    for node_id in ready {
      let node = workflow.get_node(node_id).unwrap().clone();
      let upstream_ids: Vec<String> = graph.upstream(node_id).iter().cloned().collect();
      let is_join = graph.is_join_point(node_id);

      // Gather upstream data (using output from completed tasks)
      let upstream_data: HashMap<String, serde_json::Value> = upstream_ids
        .iter()
        .filter_map(|id| completed.get(id).map(|r| (id.clone(), r.output.clone())))
        .collect();

      // Generate task_id
      let task_id = uuid::Uuid::new_v4().to_string();

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

      // Resolve inputs (template rendering)
      let resolved_input = match &node.node_type {
        NodeType::Component(locked) => {
          let resolved_strings =
            resolve_inputs(&node.node_id, &node.inputs, &upstream_data, is_join)?;
          let schema = extract_schema_types(&locked.input_schema);
          let resolved = coerce_inputs(&node.node_id, &resolved_strings, &schema)?;
          serde_json::to_value(&resolved).unwrap_or(serde_json::Value::Null)
        }
        NodeType::Join { .. } => input.clone(),
        _ => serde_json::Value::Null,
      };

      // Emit task_started event (for persistence)
      info!(
        execution_id = %execution_id,
        task_id = %task_id,
        node_id = %node_id,
        input = %input,
        resolved_input = %resolved_input,
        "task_started"
      );

      let task_input = TaskInput {
        task_id,
        execution_id: execution_id.to_string(),
        node_id: node_id.clone(),
        input,
        resolved_input,
        timeout_ms: node.timeout_ms,
      };

      // Load component if needed
      let component = match &node.node_type {
        NodeType::Component(locked) => Some(self.load_component(locked)?),
        _ => None,
      };

      let task_executor = TaskExecutor::new(self.engine.clone());
      let cancel = cancel.clone();
      let task_id_clone = task_input.task_id.clone();
      let node_id_clone = node_id.clone();

      // Spawn the task execution
      handles.push(tokio::spawn(async move {
        let task_id = task_id_clone;
        let node_id = node_id_clone;
        match node.node_type {
          NodeType::Component(_) => {
            let component = component.unwrap();
            task_executor
              .execute(&component, task_input, cancel)
              .await
              .map_err(|e| (task_id, node_id, e))
          }
          NodeType::Join { .. } => Ok(task_executor.execute_join(task_input)),
          NodeType::Trigger(_) => Err((
            task_id,
            node_id,
            fuschia_task_executor::TaskExecutionError::UnsupportedTaskType {
              message: "trigger nodes should not be executed".to_string(),
            },
          )),
          NodeType::Loop(_) => Err((
            task_id,
            node_id,
            fuschia_task_executor::TaskExecutionError::UnsupportedTaskType {
              message: "loop nodes not yet implemented".to_string(),
            },
          )),
        }
      }));
    }

    Ok(handles)
  }

  /// Load and compile a component, using the cache if available.
  fn load_component(&self, locked: &LockedComponent) -> Result<Component, ExecutionError> {
    let key = ComponentKey::new(&locked.name, &locked.version);
    let sanitized_name = locked.name.replace('/', "--");
    let dir_name = format!("{}--{}", sanitized_name, locked.version);
    let wasm_path = self
      .config
      .component_base_path
      .join(&dir_name)
      .join("component.wasm");
    self
      .component_cache
      .get_or_compile(&self.engine, &key, &wasm_path)
  }

  /// Get a reference to the wasmtime engine.
  pub fn engine(&self) -> &Engine {
    &self.engine
  }

  /// Clear the component cache.
  pub fn clear_cache(&self) {
    self.component_cache.clear();
  }

  /// Find all trigger nodes in the workflow.
  fn find_trigger_nodes(&self, workflow: &Workflow) -> Vec<String> {
    workflow
      .nodes
      .iter()
      .filter(|(_, node)| matches!(node.node_type, NodeType::Trigger(_)))
      .map(|(id, _)| id.clone())
      .collect()
  }

  /// Validate the workflow graph.
  fn validate_workflow(&self, workflow: &Workflow) -> Result<(), ExecutionError> {
    let graph = workflow.graph();
    let entry_points = graph.entry_points();

    for entry_id in entry_points {
      let node = workflow
        .get_node(entry_id)
        .ok_or_else(|| ExecutionError::InvalidGraph {
          message: format!("entry point '{}' not found in nodes", entry_id),
        })?;

      if !matches!(node.node_type, NodeType::Trigger(_)) {
        return Err(ExecutionError::InvalidGraph {
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
