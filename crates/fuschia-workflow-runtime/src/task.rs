//! Task execution.

use std::collections::HashMap;

use fuschia_task_host::{Context, TaskHostState, execute_task};
use fuschia_workflow::{LockedComponent, Node, NodeType};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};
use wasmtime::component::Component;

use crate::cache::ComponentKey;
use crate::error::RuntimeError;
use crate::input::{coerce_inputs, extract_schema_types, resolve_inputs};
use crate::runtime::WorkflowRuntime;

/// Result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
  /// Unique task ID.
  pub task_id: String,
  /// Node ID this task executed.
  pub node_id: String,
  /// Input from upstream node(s) or caller.
  pub input: serde_json::Value,
  /// Input after template resolution.
  pub resolved_input: serde_json::Value,
  /// Task output.
  pub output: serde_json::Value,
}

/// A handle to a task execution.
///
/// Call `.wait()` to run the execution and get the result.
pub struct TaskExecution<'a> {
  runtime: &'a WorkflowRuntime,
  execution_id: String,
  task_id: String,
  node: Node,
  input: serde_json::Value,
  cancel: CancellationToken,
}

impl<'a> TaskExecution<'a> {
  pub(crate) fn new(
    runtime: &'a WorkflowRuntime,
    execution_id: String,
    task_id: String,
    node: Node,
    input: serde_json::Value,
    cancel: CancellationToken,
  ) -> Self {
    Self {
      runtime,
      execution_id,
      task_id,
      node,
      input,
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
    if self.cancel.is_cancelled() {
      return Err(RuntimeError::Cancelled);
    }

    // Resolve inputs based on node type
    let resolved_input = self.resolve_input()?;

    // Emit task_started event (for persistence)
    info!(
      execution_id = %self.execution_id,
      task_id = %self.task_id,
      node_id = %self.node.node_id,
      input = %self.input,
      resolved_input = %resolved_input,
      "task_started"
    );

    let result = self.execute_inner(&resolved_input).await;

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

  /// Resolve input templates.
  fn resolve_input(&self) -> Result<serde_json::Value, RuntimeError> {
    match &self.node.node_type {
      NodeType::Component(locked) => {
        // Build upstream data map - for standalone execution, use "input" as the key
        let mut upstream_data = HashMap::new();
        upstream_data.insert("input".to_string(), self.input.clone());

        let resolved_strings =
          resolve_inputs(&self.node.node_id, &self.node.inputs, &upstream_data, false).map_err(
            |e| RuntimeError::InputResolution {
              node_id: self.node.node_id.clone(),
              message: e.to_string(),
            },
          )?;

        let schema = extract_schema_types(&locked.input_schema);
        let resolved =
          coerce_inputs(&self.node.node_id, &resolved_strings, &schema).map_err(|e| {
            RuntimeError::InputResolution {
              node_id: self.node.node_id.clone(),
              message: e.to_string(),
            }
          })?;

        Ok(serde_json::to_value(&resolved).unwrap_or(serde_json::Value::Null))
      }
      NodeType::Join { .. } => {
        // Join nodes pass through input as-is
        Ok(self.input.clone())
      }
      NodeType::Trigger(_) => {
        // Triggers shouldn't be executed directly
        Err(RuntimeError::InvalidGraph {
          message: format!(
            "trigger node '{}' cannot be executed directly",
            self.node.node_id
          ),
        })
      }
      NodeType::Loop(_) => Err(RuntimeError::InvalidGraph {
        message: format!("loop nodes not yet implemented: {}", self.node.node_id),
      }),
    }
  }

  /// Execute the task.
  async fn execute_inner(
    &self,
    resolved_input: &serde_json::Value,
  ) -> Result<TaskResult, RuntimeError> {
    match &self.node.node_type {
      NodeType::Component(locked) => self.execute_component(locked, resolved_input).await,
      NodeType::Join { .. } => Ok(TaskResult {
        task_id: self.task_id.clone(),
        node_id: self.node.node_id.clone(),
        input: self.input.clone(),
        resolved_input: resolved_input.clone(),
        output: self.input.clone(),
      }),
      NodeType::Trigger(_) | NodeType::Loop(_) => {
        // Already handled in resolve_input
        unreachable!()
      }
    }
  }

  /// Execute a component node.
  async fn execute_component(
    &self,
    locked: &LockedComponent,
    resolved_input: &serde_json::Value,
  ) -> Result<TaskResult, RuntimeError> {
    // Load component
    let component = self.load_component(locked)?;

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
      &self.runtime.engine,
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

  /// Load and compile a component.
  fn load_component(&self, locked: &LockedComponent) -> Result<Component, RuntimeError> {
    let key = ComponentKey::new(&locked.name, &locked.version);
    let sanitized_name = locked.name.replace('/', "--");
    let dir_name = format!("{}--{}", sanitized_name, locked.version);
    let wasm_path = self
      .runtime
      .config
      .component_base_path
      .join(&dir_name)
      .join("component.wasm");

    self
      .runtime
      .component_cache
      .get_or_compile(&self.runtime.engine, &key, &wasm_path)
      .map_err(|e| RuntimeError::InvalidGraph {
        message: format!("failed to load component: {}", e),
      })
  }
}
