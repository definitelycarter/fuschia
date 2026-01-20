//! Workflow runtime.

use std::path::PathBuf;
use std::sync::Arc;

use fuschia_workflow::{Node, Workflow};
use tokio_util::sync::CancellationToken;
use wasmtime::Engine;

use crate::cache::ComponentCache;
use crate::error::RuntimeError;
use crate::execution::WorkflowExecution;
use crate::task::TaskExecution;

/// Configuration for the workflow runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
  /// Base path for resolving component wasm files.
  pub component_base_path: PathBuf,
}

/// The workflow runtime.
///
/// Owns a resolved workflow and provides methods to execute the entire workflow
/// or individual nodes.
pub struct WorkflowRuntime {
  pub(crate) engine: Arc<Engine>,
  pub(crate) component_cache: ComponentCache,
  pub(crate) config: RuntimeConfig,
  pub(crate) workflow: Workflow,
}

impl WorkflowRuntime {
  /// Create a new workflow runtime.
  ///
  /// # Arguments
  /// * `config` - Runtime configuration
  /// * `workflow` - The resolved workflow to execute
  pub fn new(config: RuntimeConfig, workflow: Workflow) -> Result<Self, RuntimeError> {
    let mut wasmtime_config = wasmtime::Config::new();
    wasmtime_config.async_support(true);
    wasmtime_config.epoch_interruption(true);

    let engine = Engine::new(&wasmtime_config).map_err(|e| RuntimeError::InvalidGraph {
      message: format!("failed to create wasmtime engine: {}", e),
    })?;

    Ok(Self {
      engine: Arc::new(engine),
      component_cache: ComponentCache::new(),
      config,
      workflow,
    })
  }

  /// Execute the entire workflow with the given trigger payload.
  ///
  /// Returns a `WorkflowExecution` handle. Call `.wait()` to run the execution
  /// and get the result.
  pub fn execute_workflow(
    &self,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> WorkflowExecution<'_> {
    let execution_id = uuid::Uuid::new_v4().to_string();

    WorkflowExecution::new(self, execution_id, payload, cancel)
  }

  /// Execute a single node by ID.
  ///
  /// The input will have templates resolved against it before execution.
  /// Returns a `TaskExecution` handle. Call `.wait()` to run the execution
  /// and get the result.
  ///
  /// # Errors
  /// Returns an error if the node ID is not found in the workflow.
  pub fn execute_node(
    &self,
    node_id: &str,
    input: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<TaskExecution<'_>, RuntimeError> {
    let node = self
      .workflow
      .nodes
      .get(node_id)
      .ok_or_else(|| RuntimeError::NodeNotFound {
        node_id: node_id.to_string(),
      })?;

    let execution_id = uuid::Uuid::new_v4().to_string();
    let task_id = uuid::Uuid::new_v4().to_string();

    Ok(TaskExecution::new(
      self,
      execution_id,
      task_id,
      node.clone(),
      input,
      cancel,
    ))
  }

  /// Get a reference to the workflow.
  pub fn workflow(&self) -> &Workflow {
    &self.workflow
  }

  /// Look up a node by ID.
  pub(crate) fn get_node(&self, node_id: &str) -> Option<&Node> {
    self.workflow.nodes.get(node_id)
  }

  /// Clear the component cache.
  pub fn clear_cache(&self) {
    self.component_cache.clear();
  }
}
