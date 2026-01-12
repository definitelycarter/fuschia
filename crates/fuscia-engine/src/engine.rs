//! Workflow execution engine.
//!
//! The `WorkflowEngine` handles graph traversal and task execution for workflows.
//! It executes nodes in parallel when their dependencies are satisfied.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use fuscia_task_host::{Context, TaskHostState, execute_task};
use fuscia_workflow::{NodeType, Workflow};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use wasmtime::Engine;

use crate::cache::{ComponentCache, ComponentKey};
use crate::error::ExecutionError;
use crate::input::resolve_inputs;

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
pub struct WorkflowEngine {
  wasmtime_engine: Arc<Engine>,
  component_cache: ComponentCache,
  config: EngineConfig,
}

impl WorkflowEngine {
  /// Create a new workflow engine.
  pub fn new(config: EngineConfig) -> Result<Self, ExecutionError> {
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
    })
  }

  /// Execute a workflow with the given trigger payload.
  ///
  /// The `trigger_node_id` specifies which trigger node initiated this execution.
  /// The payload is assigned to that trigger node and flows downstream.
  pub async fn execute(
    &self,
    workflow: &Workflow,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<ExecutionResult, ExecutionError> {
    let execution_id = uuid::Uuid::new_v4().to_string();
    let graph = workflow.graph();

    // Validate the workflow graph
    self.validate_workflow(workflow)?;

    // Completed node results
    let completed: Arc<Mutex<HashMap<String, NodeResult>>> = Arc::new(Mutex::new(HashMap::new()));

    // Find trigger nodes and store the payload for each
    // For now, all triggers receive the same payload (in the future, we may
    // want to specify which trigger initiated the execution)
    let trigger_nodes = self.find_trigger_nodes(workflow);
    if trigger_nodes.is_empty() {
      return Err(ExecutionError::InvalidGraph {
        message: "workflow has no trigger nodes".to_string(),
      });
    }

    // Store the trigger payload for each trigger node
    {
      let mut completed_guard = completed.lock().await;
      for trigger_id in &trigger_nodes {
        completed_guard.insert(
          trigger_id.clone(),
          NodeResult {
            node_id: trigger_id.clone(),
            data: payload.clone(),
          },
        );
      }
    }

    // Find initially ready nodes (downstream of entry)
    let mut ready = self.find_ready_nodes(workflow, &completed).await;

    while !ready.is_empty() {
      // Check for cancellation before starting batch
      if cancel.is_cancelled() {
        return Err(ExecutionError::Cancelled);
      }

      // Execute ready nodes in parallel
      let handles: Vec<_> = ready
        .into_iter()
        .map(|node_id| {
          let engine = self.wasmtime_engine.clone();
          let workflow = workflow.clone();
          let completed = completed.clone();
          let cancel = cancel.clone();
          let execution_id = execution_id.clone();
          let component_cache = &self.component_cache;
          let config = &self.config;

          // Get upstream data for this node
          let upstream_ids: Vec<String> = graph.upstream(&node_id).iter().cloned().collect();
          let is_join = graph.is_join_point(&node_id);

          // Prepare the task
          let node = workflow.get_node(&node_id).unwrap().clone();

          // Get component info and compile
          let component_result = match &node.node_type {
            NodeType::Trigger(_) => {
              // Trigger nodes should already be completed with the payload
              // This case shouldn't be reached in normal execution
              return tokio::spawn(async move {
                Err(ExecutionError::InvalidGraph {
                  message: format!("trigger node '{}' should not be in ready queue", node_id),
                })
              });
            }
            NodeType::Component(locked) => {
              let key = ComponentKey::new(&locked.name, &locked.version);
              let wasm_path = config
                .component_base_path
                .join(&locked.name)
                .join(&locked.version)
                .join("component.wasm");
              component_cache.get_or_compile(&engine, &key, &wasm_path)
            }
            NodeType::Join { .. } => {
              // Join nodes don't execute components, they just merge data
              return tokio::spawn(async move {
                let completed_guard = completed.lock().await;
                let mut merged = serde_json::Map::new();
                for upstream_id in &upstream_ids {
                  if let Some(result) = completed_guard.get(upstream_id) {
                    merged.insert(upstream_id.clone(), result.data.clone());
                  }
                }
                Ok(NodeResult {
                  node_id,
                  data: serde_json::Value::Object(merged),
                })
              });
            }
            NodeType::Loop(_) => {
              // Loop nodes are deferred for now
              return tokio::spawn(async move {
                Err(ExecutionError::InvalidGraph {
                  message: format!("loop nodes not yet implemented: {}", node_id),
                })
              });
            }
          };

          let component = match component_result {
            Ok(c) => c,
            Err(e) => {
              return tokio::spawn(async move { Err(e) });
            }
          };

          tokio::spawn(async move {
            // Check cancellation
            if cancel.is_cancelled() {
              return Err(ExecutionError::Cancelled);
            }

            // Gather upstream data
            let upstream_data = {
              let completed_guard = completed.lock().await;
              upstream_ids
                .iter()
                .filter_map(|id| {
                  completed_guard
                    .get(id)
                    .map(|r| (id.clone(), r.data.clone()))
                })
                .collect::<HashMap<_, _>>()
            };

            // Resolve inputs
            let resolved_inputs = resolve_inputs(&node_id, &node.inputs, &upstream_data, is_join)?;

            // Create host state
            let state = TaskHostState::new(execution_id.clone(), node_id.clone());

            // Create context
            let ctx = Context {
              execution_id: execution_id.clone(),
              node_id: node_id.clone(),
              task_id: uuid::Uuid::new_v4().to_string(),
            };

            // Serialize input data
            let input_data = serde_json::to_string(&resolved_inputs).map_err(|e| {
              ExecutionError::InputResolution {
                node_id: node_id.clone(),
                message: format!("failed to serialize inputs: {}", e),
              }
            })?;

            // Calculate epoch deadline based on timeout
            let epoch_deadline = node.timeout_ms.map(|ms| ms / 10); // Rough conversion

            // Execute the task
            let result = execute_task(&engine, &component, state, ctx, input_data, epoch_deadline)
              .await
              .map_err(|e| ExecutionError::ComponentExecution {
                node_id: node_id.clone(),
                source: e,
              })?;

            // Parse output data
            let output_data: serde_json::Value = serde_json::from_str(&result.output.data)
              .map_err(|e| ExecutionError::ComponentExecution {
                node_id: node_id.clone(),
                source: fuscia_host::HostError::component_error(format!(
                  "invalid output JSON: {}",
                  e
                )),
              })?;

            Ok(NodeResult {
              node_id,
              data: output_data,
            })
          })
        })
        .collect();

      // Wait for all tasks, checking for cancellation
      let results = tokio::select! {
          results = futures::future::join_all(handles) => results,
          _ = cancel.cancelled() => return Err(ExecutionError::Cancelled),
      };

      // Process results
      for result in results {
        let node_result = result.map_err(|e| ExecutionError::InvalidGraph {
          message: format!("task join error: {}", e),
        })??;

        let mut completed_guard = completed.lock().await;
        completed_guard.insert(node_result.node_id.clone(), node_result);
      }

      // Find newly ready nodes
      ready = self.find_ready_nodes(workflow, &completed).await;
    }

    // Build final result
    let completed_guard = completed.lock().await;
    Ok(ExecutionResult {
      execution_id,
      node_results: completed_guard.clone(),
    })
  }

  /// Find nodes that are ready to execute (all upstream nodes completed).
  async fn find_ready_nodes(
    &self,
    workflow: &Workflow,
    completed: &Arc<Mutex<HashMap<String, NodeResult>>>,
  ) -> Vec<String> {
    let graph = workflow.graph();
    let completed_guard = completed.lock().await;

    workflow
      .nodes
      .keys()
      .filter(|id| !completed_guard.contains_key(*id))
      .filter(|id| {
        graph
          .upstream(id)
          .iter()
          .all(|up| completed_guard.contains_key(up))
      })
      .cloned()
      .collect()
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
  ///
  /// Checks:
  /// - All entry points (nodes with no incoming edges) must be trigger nodes
  /// - Non-trigger nodes with no incoming edges are orphans (error)
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
