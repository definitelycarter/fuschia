//! Trigger component execution.

use std::sync::Arc;

use fuschia_trigger_host::{Event, TriggerHostState, TriggerResult, execute_trigger};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};
use wasmtime::Engine;
use wasmtime::component::Component;

use crate::error::RuntimeError;

/// Input required to execute a trigger component.
pub struct TriggerInput {
  /// Execution ID this trigger belongs to.
  pub execution_id: String,
  /// Node ID being executed.
  pub node_id: String,
  /// The trigger event (poll or webhook).
  pub event: Event,
  /// Timeout in milliseconds.
  pub timeout_ms: Option<u64>,
}

/// Executes trigger wasm components against the wasmtime runtime.
pub struct TriggerExecutor {
  engine: Arc<Engine>,
}

impl TriggerExecutor {
  /// Create a new trigger executor with the given wasmtime engine.
  pub fn new(engine: Arc<Engine>) -> Self {
    Self { engine }
  }

  /// Execute a trigger wasm component.
  #[instrument(
    name = "trigger_execute",
    skip(self, component, input, cancel),
    fields(
      execution_id = %input.execution_id,
      node_id = %input.node_id,
    )
  )]
  pub async fn execute(
    &self,
    component: &Component,
    input: TriggerInput,
    cancel: CancellationToken,
  ) -> Result<TriggerResult, RuntimeError> {
    info!("trigger started");

    let result = self.execute_inner(component, input, cancel).await;

    match &result {
      Ok(trigger_result) => {
        info!(status = ?trigger_result.status, "trigger completed");
      }
      Err(e) => {
        error!(error = %e, "trigger failed");
      }
    }

    result
  }

  /// Inner execution logic.
  async fn execute_inner(
    &self,
    component: &Component,
    input: TriggerInput,
    cancel: CancellationToken,
  ) -> Result<TriggerResult, RuntimeError> {
    if cancel.is_cancelled() {
      return Err(RuntimeError::Cancelled);
    }

    let state = TriggerHostState::new(input.execution_id, input.node_id);
    let epoch_deadline = input.timeout_ms.map(|ms| ms / 10);

    execute_trigger(&self.engine, component, state, input.event, epoch_deadline)
      .await
      .map_err(|e| RuntimeError::ComponentExecution { source: e })
  }
}
