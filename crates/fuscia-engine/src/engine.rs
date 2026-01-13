//! Workflow execution engine.
//!
//! The `WorkflowEngine` handles graph traversal and task execution for workflows.
//! It executes nodes in parallel when their dependencies are satisfied.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use fuscia_task_host::{Context, TaskHostState, execute_task};
use fuscia_workflow::{LockedComponent, Node, NodeType, Workflow};
use tokio_util::sync::CancellationToken;
use wasmtime::Engine;
use wasmtime::component::Component;

use crate::cache::{ComponentCache, ComponentKey};
use crate::error::ExecutionError;
use crate::events::{ExecutionEvent, ExecutionNotifier, NoopNotifier};
use crate::input::{coerce_inputs, extract_schema_types, resolve_inputs};

/// Result of a single node execution.
#[derive(Debug, Clone)]
pub struct NodeResult {
  pub node_id: String,
  pub data: serde_json::Value,
}

/// Result of a complete workflow execution.
#[derive(Debug)]
pub struct ExecutionResult {
  pub execution_id: String,
  pub node_results: HashMap<String, NodeResult>,
}

/// Configuration for the workflow engine.
pub struct EngineConfig {
  /// Base path for resolving component wasm files.
  pub component_base_path: PathBuf,
}

/// The workflow execution engine.
///
/// Generic over `N: ExecutionNotifier` to allow different notification strategies.
/// Use `WorkflowEngine::new()` for a default engine with no-op notifications,
/// or `WorkflowEngine::with_notifier()` to provide a custom notifier.
pub struct WorkflowEngine<N: ExecutionNotifier = NoopNotifier> {
  wasmtime_engine: Arc<Engine>,
  component_cache: ComponentCache,
  config: EngineConfig,
  notifier: N,
}

impl WorkflowEngine<NoopNotifier> {
  /// Create a new workflow engine with no-op notifications.
  ///
  /// Events are discarded. Use `with_notifier` if you need to observe events.
  pub fn new(config: EngineConfig) -> Result<Self, ExecutionError> {
    Self::with_notifier(config, NoopNotifier)
  }
}

impl<N: ExecutionNotifier> WorkflowEngine<N> {
  /// Create a new workflow engine with a custom notifier.
  pub fn with_notifier(config: EngineConfig, notifier: N) -> Result<Self, ExecutionError> {
    let mut wasmtime_config = wasmtime::Config::new();
    wasmtime_config.async_support(true);
    wasmtime_config.epoch_interruption(true);

    let wasmtime_engine =
      Engine::new(&wasmtime_config).map_err(|e| ExecutionError::InvalidGraph {
        message: format!("failed to create wasmtime engine: {}", e),
      })?;

    Ok(Self {
      wasmtime_engine: Arc::new(wasmtime_engine),
      component_cache: ComponentCache::new(),
      config,
      notifier,
    })
  }

  /// Execute a workflow with the given trigger payload.
  pub async fn execute(
    &self,
    workflow: &Workflow,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<ExecutionResult, ExecutionError> {
    let execution_id = uuid::Uuid::new_v4().to_string();

    // Validate the workflow graph
    self.validate_workflow(workflow)?;

    // Emit workflow started event
    self.notifier.notify(ExecutionEvent::WorkflowStarted {
      execution_id: execution_id.clone(),
      workflow_id: workflow.workflow_id.clone(),
    });

    // Initialize completed results with trigger node payloads
    let mut completed = self.initialize_triggers(workflow, payload)?;

    // Execute nodes until no more are ready
    let result = self
      .run_execution_loop(workflow, &mut completed, &execution_id, &cancel)
      .await;

    // Emit final event based on result
    match &result {
      Ok(_) => {
        self.notifier.notify(ExecutionEvent::WorkflowCompleted {
          execution_id: execution_id.clone(),
        });
      }
      Err(e) => {
        self.notifier.notify(ExecutionEvent::WorkflowFailed {
          execution_id: execution_id.clone(),
          error: e.to_string(),
        });
      }
    }

    result
  }

  /// Run the main execution loop.
  async fn run_execution_loop(
    &self,
    workflow: &Workflow,
    completed: &mut HashMap<String, NodeResult>,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<ExecutionResult, ExecutionError> {
    loop {
      if cancel.is_cancelled() {
        return Err(ExecutionError::Cancelled);
      }

      let ready = self.find_ready_nodes(workflow, completed);
      if ready.is_empty() {
        break;
      }

      // Emit node started events
      for node_id in &ready {
        self.notifier.notify(ExecutionEvent::NodeStarted {
          execution_id: execution_id.to_string(),
          node_id: node_id.clone(),
        });
      }

      let handles = self.execute_ready_nodes(workflow, &ready, completed, execution_id, cancel)?;

      // Wait for all tasks
      let results = tokio::select! {
          results = futures::future::join_all(handles) => results,
          _ = cancel.cancelled() => return Err(ExecutionError::Cancelled),
      };

      // Process results and emit events
      for result in results {
        let join_result = result.map_err(|e| ExecutionError::InvalidGraph {
          message: format!("task join error: {}", e),
        })?;

        match join_result {
          Ok(node_result) => {
            self.notifier.notify(ExecutionEvent::NodeCompleted {
              execution_id: execution_id.to_string(),
              node_id: node_result.node_id.clone(),
              data: node_result.data.clone(),
            });
            completed.insert(node_result.node_id.clone(), node_result);
          }
          Err(e) => {
            // Extract node_id from error if available
            let node_id = match &e {
              ExecutionError::InputResolution { node_id, .. } => node_id.clone(),
              ExecutionError::ComponentExecution { node_id, .. } => node_id.clone(),
              ExecutionError::Timeout { node_id } => node_id.clone(),
              ExecutionError::ComponentLoad { node_id, .. } => node_id.clone(),
              _ => "unknown".to_string(),
            };
            self.notifier.notify(ExecutionEvent::NodeFailed {
              execution_id: execution_id.to_string(),
              node_id,
              error: e.to_string(),
            });
            return Err(e);
          }
        }
      }
    }

    Ok(ExecutionResult {
      execution_id: execution_id.to_string(),
      node_results: completed.clone(),
    })
  }

  /// Initialize trigger nodes with the payload.
  fn initialize_triggers(
    &self,
    workflow: &Workflow,
    payload: serde_json::Value,
  ) -> Result<HashMap<String, NodeResult>, ExecutionError> {
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
        NodeResult {
          node_id: trigger_id,
          data: payload.clone(),
        },
      );
    }
    Ok(completed)
  }

  /// Find nodes that are ready to execute (all upstream nodes completed).
  fn find_ready_nodes(
    &self,
    workflow: &Workflow,
    completed: &HashMap<String, NodeResult>,
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

  /// Spawn tasks to execute all ready nodes.
  fn execute_ready_nodes(
    &self,
    workflow: &Workflow,
    ready: &[String],
    completed: &HashMap<String, NodeResult>,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<Vec<tokio::task::JoinHandle<Result<NodeResult, ExecutionError>>>, ExecutionError> {
    let graph = workflow.graph();
    let mut handles = Vec::with_capacity(ready.len());

    for node_id in ready {
      let node = workflow.get_node(node_id).unwrap().clone();
      let upstream_ids: Vec<String> = graph.upstream(node_id).iter().cloned().collect();
      let is_join = graph.is_join_point(node_id);

      // Gather upstream data
      let upstream_data: HashMap<String, serde_json::Value> = upstream_ids
        .iter()
        .filter_map(|id| completed.get(id).map(|r| (id.clone(), r.data.clone())))
        .collect();

      let handle = match &node.node_type {
        NodeType::Trigger(_) => {
          let node_id = node_id.clone();
          tokio::spawn(async move {
            Err(ExecutionError::InvalidGraph {
              message: format!("trigger node '{}' should not be in ready queue", node_id),
            })
          })
        }
        NodeType::Component(locked) => {
          let locked = locked.clone();
          self.spawn_component_node(node, &locked, upstream_data, is_join, execution_id, cancel)?
        }
        NodeType::Join { .. } => Self::spawn_join_node(node_id.clone(), upstream_data),
        NodeType::Loop(_) => {
          let node_id = node_id.clone();
          tokio::spawn(async move {
            Err(ExecutionError::InvalidGraph {
              message: format!("loop nodes not yet implemented: {}", node_id),
            })
          })
        }
      };

      handles.push(handle);
    }

    Ok(handles)
  }

  /// Spawn a task to execute a component node.
  fn spawn_component_node(
    &self,
    node: Node,
    locked: &LockedComponent,
    upstream_data: HashMap<String, serde_json::Value>,
    is_join: bool,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<tokio::task::JoinHandle<Result<NodeResult, ExecutionError>>, ExecutionError> {
    // Compile component
    let key = ComponentKey::new(&locked.name, &locked.version);
    let wasm_path = self
      .config
      .component_base_path
      .join(&locked.name)
      .join(&locked.version)
      .join("component.wasm");
    let component = self
      .component_cache
      .get_or_compile(&self.wasmtime_engine, &key, &wasm_path)?;

    let engine = self.wasmtime_engine.clone();
    let input_schema = locked.input_schema.clone();
    let execution_id = execution_id.to_string();
    let cancel = cancel.clone();

    Ok(tokio::spawn(async move {
      execute_component(
        node,
        component,
        engine,
        upstream_data,
        is_join,
        input_schema,
        execution_id,
        cancel,
      )
      .await
    }))
  }

  /// Spawn a task to execute a join node.
  fn spawn_join_node(
    node_id: String,
    upstream_data: HashMap<String, serde_json::Value>,
  ) -> tokio::task::JoinHandle<Result<NodeResult, ExecutionError>> {
    tokio::spawn(async move {
      let mut merged = serde_json::Map::new();
      for (id, data) in upstream_data {
        merged.insert(id, data);
      }
      Ok(NodeResult {
        node_id,
        data: serde_json::Value::Object(merged),
      })
    })
  }

  /// Get a reference to the wasmtime engine.
  pub fn wasmtime_engine(&self) -> &Engine {
    &self.wasmtime_engine
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

/// Execute a component node (runs inside spawned task).
async fn execute_component(
  node: Node,
  component: Component,
  engine: Arc<Engine>,
  upstream_data: HashMap<String, serde_json::Value>,
  is_join: bool,
  input_schema: serde_json::Value,
  execution_id: String,
  cancel: CancellationToken,
) -> Result<NodeResult, ExecutionError> {
  let node_id = node.node_id.clone();

  if cancel.is_cancelled() {
    return Err(ExecutionError::Cancelled);
  }

  // Resolve inputs (template rendering)
  let resolved_strings = resolve_inputs(&node_id, &node.inputs, &upstream_data, is_join)?;

  // Coerce inputs to typed values based on component schema
  let schema = extract_schema_types(&input_schema);
  let resolved_inputs = coerce_inputs(&node_id, &resolved_strings, &schema)?;

  // Create host state and context
  let state = TaskHostState::new(execution_id.clone(), node_id.clone());
  let ctx = Context {
    execution_id: execution_id.clone(),
    node_id: node_id.clone(),
    task_id: uuid::Uuid::new_v4().to_string(),
  };

  // Serialize input data
  let input_data =
    serde_json::to_string(&resolved_inputs).map_err(|e| ExecutionError::InputResolution {
      node_id: node_id.clone(),
      message: format!("failed to serialize inputs: {}", e),
    })?;

  // Calculate epoch deadline based on timeout
  let epoch_deadline = node.timeout_ms.map(|ms| ms / 10);

  // Execute the task
  let result = execute_task(&engine, &component, state, ctx, input_data, epoch_deadline)
    .await
    .map_err(|e| ExecutionError::ComponentExecution {
      node_id: node_id.clone(),
      source: e,
    })?;

  // Parse output data
  let output_data: serde_json::Value =
    serde_json::from_str(&result.output.data).map_err(|e| ExecutionError::ComponentExecution {
      node_id: node_id.clone(),
      source: fuscia_host::HostError::component_error(format!("invalid output JSON: {}", e)),
    })?;

  Ok(NodeResult {
    node_id,
    data: output_data,
  })
}
