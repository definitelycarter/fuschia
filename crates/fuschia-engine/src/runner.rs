//! Workflow runner with channel-based triggering.
//!
//! The `WorkflowRunner` owns an mpsc channel for receiving trigger payloads
//! and executes workflows using the `WorkflowRuntime`.

use std::sync::Arc;

use fuschia_workflow_runtime::{RuntimeError, WorkflowResult, WorkflowRuntime};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// A runner that executes a workflow in response to trigger payloads.
///
/// # Usage
///
/// ```ignore
/// let runner = WorkflowRunner::new(runtime);
///
/// // Get sender for external triggers (webhooks, UI, etc.)
/// let sender = runner.sender();
///
/// // Start the execution loop
/// let cancel = CancellationToken::new();
/// runner.start(cancel).await?;
/// ```
pub struct WorkflowRunner {
  sender: mpsc::Sender<serde_json::Value>,
  receiver: mpsc::Receiver<serde_json::Value>,
  runtime: Arc<WorkflowRuntime>,
}

impl WorkflowRunner {
  /// Create a new workflow runner.
  ///
  /// # Arguments
  /// * `runtime` - The workflow runtime (owns the workflow)
  pub fn new(runtime: Arc<WorkflowRuntime>) -> Self {
    Self::with_buffer_size(runtime, 100)
  }

  /// Create a new workflow runner with a custom buffer size.
  pub fn with_buffer_size(runtime: Arc<WorkflowRuntime>, buffer_size: usize) -> Self {
    let (sender, receiver) = mpsc::channel(buffer_size);
    Self {
      sender,
      receiver,
      runtime,
    }
  }

  /// Get a sender handle for triggering workflow executions.
  ///
  /// This can be given to webhooks, UI handlers, poll tasks, etc.
  pub fn sender(&self) -> mpsc::Sender<serde_json::Value> {
    self.sender.clone()
  }

  /// Trigger a workflow execution with the given payload.
  ///
  /// This is a convenience method that sends through the channel.
  pub async fn run(&self, payload: serde_json::Value) -> Result<(), RuntimeError> {
    self
      .sender
      .send(payload)
      .await
      .map_err(|_| RuntimeError::InvalidGraph {
        message: "workflow runner channel closed".to_string(),
      })
  }

  /// Start the execution loop.
  ///
  /// This blocks until the cancellation token is triggered or the channel closes.
  /// Each received payload triggers a workflow execution.
  pub async fn start(mut self, cancel: CancellationToken) -> Result<(), RuntimeError> {
    info!(
        workflow_id = %self.runtime.workflow().workflow_id,
        workflow_name = %self.runtime.workflow().name,
        "starting workflow runner"
    );

    loop {
      tokio::select! {
          _ = cancel.cancelled() => {
              info!(
                  workflow_id = %self.runtime.workflow().workflow_id,
                  "workflow runner cancelled"
              );
              break;
          }
          payload = self.receiver.recv() => {
              match payload {
                  Some(payload) => {
                      // Create a child cancellation token for this execution
                      let exec_cancel = cancel.child_token();

                      info!(
                          workflow_id = %self.runtime.workflow().workflow_id,
                          "triggering workflow execution"
                      );

                      let execution = self.runtime.execute_workflow(payload, exec_cancel);
                      match execution.wait().await {
                          Ok(result) => {
                              info!(
                                  workflow_id = %self.runtime.workflow().workflow_id,
                                  execution_id = %result.execution_id,
                                  tasks_executed = result.task_results.len(),
                                  "workflow execution completed"
                              );
                          }
                          Err(RuntimeError::Cancelled) => {
                              info!(
                                  workflow_id = %self.runtime.workflow().workflow_id,
                                  "workflow execution cancelled"
                              );
                          }
                          Err(e) => {
                              error!(
                                  workflow_id = %self.runtime.workflow().workflow_id,
                                  error = %e,
                                  "workflow execution failed"
                              );
                          }
                      }
                  }
                  None => {
                      // Channel closed, exit loop
                      info!(
                          workflow_id = %self.runtime.workflow().workflow_id,
                          "workflow runner channel closed"
                      );
                      break;
                  }
              }
          }
      }
    }

    Ok(())
  }

  /// Execute a single workflow run synchronously (without the loop).
  ///
  /// This is useful for testing or one-shot executions.
  pub async fn execute_once(
    &self,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<WorkflowResult, RuntimeError> {
    self.runtime.execute_workflow(payload, cancel).wait().await
  }

  /// Get a reference to the runtime.
  pub fn runtime(&self) -> &WorkflowRuntime {
    &self.runtime
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use fuschia_workflow::Workflow;
  use fuschia_workflow_runtime::RuntimeConfig;
  use std::path::PathBuf;
  use std::time::Duration;

  fn create_test_workflow() -> Workflow {
    Workflow {
      workflow_id: "test-workflow".to_string(),
      name: "Test Workflow".to_string(),
      nodes: std::collections::HashMap::new(),
      edges: vec![],
      timeout_ms: None,
      max_retry_attempts: None,
    }
  }

  fn create_test_runtime() -> Arc<WorkflowRuntime> {
    let config = RuntimeConfig {
      component_base_path: PathBuf::from("/tmp/components"),
    };
    let workflow = create_test_workflow();
    Arc::new(WorkflowRuntime::new(config, workflow).unwrap())
  }

  #[tokio::test]
  async fn test_runner_creation() {
    let runtime = create_test_runtime();
    let runner = WorkflowRunner::new(runtime);

    assert_eq!(runner.runtime().workflow().workflow_id, "test-workflow");
  }

  #[tokio::test]
  async fn test_sender_cloning() {
    let runtime = create_test_runtime();
    let runner = WorkflowRunner::new(runtime);

    let sender1 = runner.sender();
    let sender2 = runner.sender();

    // Both senders should work
    assert!(!sender1.is_closed());
    assert!(!sender2.is_closed());
  }

  #[tokio::test]
  async fn test_run_sends_to_channel() {
    let runtime = create_test_runtime();
    let mut runner = WorkflowRunner::new(runtime);

    // Send a payload
    runner
      .run(serde_json::json!({"test": "data"}))
      .await
      .unwrap();

    // Should be receivable
    let received = runner.receiver.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap()["test"], "data");
  }

  #[tokio::test]
  async fn test_cancellation() {
    let runtime = create_test_runtime();
    let runner = WorkflowRunner::new(runtime);

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Start runner in background
    let handle = tokio::spawn(async move { runner.start(cancel_clone).await });

    // Cancel after a short delay
    tokio::time::sleep(Duration::from_millis(10)).await;
    cancel.cancel();

    // Runner should exit cleanly
    let result = handle.await.unwrap();
    assert!(result.is_ok());
  }
}
