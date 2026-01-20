//! Task executor implementation.

use std::sync::Arc;

use fuschia_task_host::{Context, TaskHostState, execute_task};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};
use wasmtime::Engine;
use wasmtime::component::Component;

use crate::error::TaskExecutionError;
use crate::result::TaskResult;

/// Input required to execute a task.
pub struct TaskInput {
  /// Unique task ID.
  pub task_id: String,
  /// Execution ID this task belongs to.
  pub execution_id: String,
  /// Node ID being executed.
  pub node_id: String,
  /// Input from upstream node(s).
  pub input: serde_json::Value,
  /// Input after template resolution.
  pub resolved_input: serde_json::Value,
  /// Timeout in milliseconds.
  pub timeout_ms: Option<u64>,
}

/// Executes tasks against the wasm runtime.
pub struct TaskExecutor {
  engine: Arc<Engine>,
}

impl TaskExecutor {
  /// Create a new task executor with the given wasmtime engine.
  pub fn new(engine: Arc<Engine>) -> Self {
    Self { engine }
  }

  /// Execute a component task.
  #[instrument(
    name = "task_execute",
    skip(self, component, input, cancel),
    fields(
      execution_id = %input.execution_id,
      task_id = %input.task_id,
      node_id = %input.node_id,
    )
  )]
  pub async fn execute(
    &self,
    component: &Component,
    input: TaskInput,
    cancel: CancellationToken,
  ) -> Result<TaskResult, TaskExecutionError> {
    info!(
      input = %input.input,
      resolved_input = %input.resolved_input,
      "task started"
    );

    let result = self.execute_inner(component, &input, cancel).await;

    match &result {
      Ok(task_result) => {
        info!(output = %task_result.output, "task completed");
      }
      Err(e) => {
        error!(error = %e, "task failed");
      }
    }

    result
  }

  /// Inner execution logic.
  async fn execute_inner(
    &self,
    component: &Component,
    input: &TaskInput,
    cancel: CancellationToken,
  ) -> Result<TaskResult, TaskExecutionError> {
    if cancel.is_cancelled() {
      return Err(TaskExecutionError::Cancelled);
    }

    // Create host state and context
    let state = TaskHostState::new(input.execution_id.clone(), input.node_id.clone());
    let ctx = Context {
      execution_id: input.execution_id.clone(),
      node_id: input.node_id.clone(),
      task_id: input.task_id.clone(),
    };

    // Serialize resolved input for the component
    let input_data = serde_json::to_string(&input.resolved_input).map_err(|e| {
      TaskExecutionError::InputSerialization {
        message: e.to_string(),
      }
    })?;

    // Calculate epoch deadline based on timeout
    let epoch_deadline = input.timeout_ms.map(|ms| ms / 10);

    // Execute the task
    let result = execute_task(
      &self.engine,
      component,
      state,
      ctx,
      input_data,
      epoch_deadline,
    )
    .await
    .map_err(|e| TaskExecutionError::ComponentExecution { source: e })?;

    // Parse output data
    let output: serde_json::Value =
      serde_json::from_str(&result.output.data).map_err(|e| TaskExecutionError::InvalidOutput {
        message: format!("invalid JSON: {}", e),
      })?;

    Ok(TaskResult {
      task_id: input.task_id.clone(),
      node_id: input.node_id.clone(),
      input: input.input.clone(),
      resolved_input: input.resolved_input.clone(),
      output,
    })
  }

  /// Execute a join task (merges upstream inputs).
  ///
  /// Join tasks don't run any wasm - they just pass through the merged input.
  #[instrument(
    name = "task_join",
    skip(self, input),
    fields(
      execution_id = %input.execution_id,
      task_id = %input.task_id,
      node_id = %input.node_id,
    )
  )]
  pub fn execute_join(&self, input: TaskInput) -> TaskResult {
    info!(input = %input.input, "join task started");

    let result = TaskResult {
      task_id: input.task_id,
      node_id: input.node_id,
      input: input.input.clone(),
      resolved_input: input.resolved_input,
      output: input.input,
    };

    info!(output = %result.output, "join task completed");

    result
  }
}
