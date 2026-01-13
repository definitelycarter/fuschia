//! Workflow execution engine.
//!
//! The `WorkflowEngine` handles graph traversal and task execution for workflows.
//! It executes nodes in parallel when their dependencies are satisfied.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use fuschia_task_host::{Context, TaskHostState, execute_task};
use fuschia_workflow::{LockedComponent, Node, NodeType, Workflow};
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
  engine: Arc<Engine>,
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
      engine: Arc::new(wasmtime_engine),
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

  /// Spawn tasks to execute all ready nodes in parallel.
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

      // Prepare execution context (load component if needed)
      let exec_ctx = self.prepare_node_execution(&node, upstream_data, is_join, execution_id)?;
      let cancel = cancel.clone();

      // Spawn the node execution
      handles.push(tokio::spawn(async move {
        run_node(node, exec_ctx, cancel).await
      }));
    }

    Ok(handles)
  }

  /// Prepare everything needed to execute a node (without actually running it).
  fn prepare_node_execution(
    &self,
    node: &Node,
    upstream_data: HashMap<String, serde_json::Value>,
    is_join: bool,
    execution_id: &str,
  ) -> Result<NodeExecutionContext, ExecutionError> {
    let component = match &node.node_type {
      NodeType::Component(locked) => Some(self.load_component(locked)?),
      _ => None,
    };

    Ok(NodeExecutionContext {
      component,
      engine: self.engine.clone(),
      upstream_data,
      is_join,
      execution_id: execution_id.to_string(),
    })
  }

  /// Load and compile a component, using the cache if available.
  ///
  /// Components are stored in the format: `{name}--{version}/component.wasm`
  /// where slashes in the name are replaced with `--`.
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

  /// Execute a single node with the given input data.
  ///
  /// This bypasses graph traversal and executes the node directly.
  /// The payload is treated as if it came from a single upstream "trigger" node.
  pub async fn execute_node(
    &self,
    node: &Node,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<NodeResult, ExecutionError> {
    let execution_id = uuid::Uuid::new_v4().to_string();
    let mut upstream_data = HashMap::new();
    upstream_data.insert("trigger".to_string(), payload);

    let exec_ctx = self.prepare_node_execution(node, upstream_data, false, &execution_id)?;
    run_node(node.clone(), exec_ctx, cancel).await
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

/// Context needed to execute a node (prepared on the main thread, used in spawned task).
struct NodeExecutionContext {
  component: Option<Component>,
  engine: Arc<Engine>,
  upstream_data: HashMap<String, serde_json::Value>,
  is_join: bool,
  execution_id: String,
}

/// Execute a node with the given context.
async fn run_node(
  node: Node,
  ctx: NodeExecutionContext,
  cancel: CancellationToken,
) -> Result<NodeResult, ExecutionError> {
  match node.node_type {
    NodeType::Trigger(_) => {
      // Triggers shouldn't be in the ready queue during execution
      Err(ExecutionError::InvalidGraph {
        message: format!("trigger node '{}' should not be executed", node.node_id),
      })
    }
    NodeType::Component(ref locked) => {
      let input_schema = locked.input_schema.clone();
      let component = ctx.component.ok_or_else(|| ExecutionError::InvalidGraph {
        message: format!("component not loaded for node '{}'", node.node_id),
      })?;
      run_component(
        node,
        component,
        ctx.engine,
        ctx.upstream_data,
        ctx.is_join,
        input_schema,
        ctx.execution_id,
        cancel,
      )
      .await
    }
    NodeType::Join { .. } => {
      let mut merged = serde_json::Map::new();
      for (id, data) in ctx.upstream_data {
        merged.insert(id, data);
      }
      Ok(NodeResult {
        node_id: node.node_id,
        data: serde_json::Value::Object(merged),
      })
    }
    NodeType::Loop(_) => Err(ExecutionError::InvalidGraph {
      message: format!("loop nodes not yet implemented: {}", node.node_id),
    }),
  }
}

/// Execute a component node.
async fn run_component(
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
      source: fuschia_host::HostError::component_error(format!("invalid output JSON: {}", e)),
    })?;

  Ok(NodeResult {
    node_id,
    data: output_data,
  })
}
