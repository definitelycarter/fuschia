//! Workflow runner with channel-based triggering.
//!
//! The `WorkflowRunner` owns an mpsc channel for receiving trigger payloads
//! and executes workflows using the `WorkflowExecutor`.

use std::sync::Arc;

use fuschia_workflow::Workflow;
use fuschia_workflow_executor::{ExecutionError, ExecutionResult, WorkflowExecutor};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// A runner that executes a workflow in response to trigger payloads.
///
/// # Usage
///
/// ```ignore
/// let runner = WorkflowRunner::new(workflow, executor);
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
  workflow: Workflow,
  executor: Arc<WorkflowExecutor>,
}

impl WorkflowRunner {
  /// Create a new workflow runner.
  ///
  /// # Arguments
  /// * `workflow` - The workflow to execute
  /// * `executor` - The workflow executor for execution
  pub fn new(workflow: Workflow, executor: Arc<WorkflowExecutor>) -> Self {
    Self::with_buffer_size(workflow, executor, 100)
  }

  /// Create a new workflow runner with a custom buffer size.
  pub fn with_buffer_size(
    workflow: Workflow,
    executor: Arc<WorkflowExecutor>,
    buffer_size: usize,
  ) -> Self {
    let (sender, receiver) = mpsc::channel(buffer_size);
    Self {
      sender,
      receiver,
      workflow,
      executor,
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
  pub async fn run(&self, payload: serde_json::Value) -> Result<(), ExecutionError> {
    self
      .sender
      .send(payload)
      .await
      .map_err(|_| ExecutionError::InvalidGraph {
        message: "workflow runner channel closed".to_string(),
      })
  }

  /// Start the execution loop.
  ///
  /// This blocks until the cancellation token is triggered or the channel closes.
  /// Each received payload triggers a workflow execution.
  pub async fn start(mut self, cancel: CancellationToken) -> Result<(), ExecutionError> {
    info!(
        workflow_id = %self.workflow.workflow_id,
        workflow_name = %self.workflow.name,
        "starting workflow runner"
    );

    loop {
      tokio::select! {
          _ = cancel.cancelled() => {
              info!(
                  workflow_id = %self.workflow.workflow_id,
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
                          workflow_id = %self.workflow.workflow_id,
                          "triggering workflow execution"
                      );

                      match self.executor.execute(&self.workflow, payload, exec_cancel).await {
                          Ok(result) => {
                              info!(
                                  workflow_id = %self.workflow.workflow_id,
                                  execution_id = %result.execution_id,
                                  tasks_executed = result.task_results.len(),
                                  "workflow execution completed"
                              );
                          }
                          Err(ExecutionError::Cancelled) => {
                              info!(
                                  workflow_id = %self.workflow.workflow_id,
                                  "workflow execution cancelled"
                              );
                          }
                          Err(e) => {
                              error!(
                                  workflow_id = %self.workflow.workflow_id,
                                  error = %e,
                                  "workflow execution failed"
                              );
                          }
                      }
                  }
                  None => {
                      // Channel closed, exit loop
                      info!(
                          workflow_id = %self.workflow.workflow_id,
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
  ) -> Result<ExecutionResult, ExecutionError> {
    self.executor.execute(&self.workflow, payload, cancel).await
  }

  /// Get a reference to the workflow.
  pub fn workflow(&self) -> &Workflow {
    &self.workflow
  }

  /// Get a reference to the executor.
  pub fn executor(&self) -> &WorkflowExecutor {
    &self.executor
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use fuschia_workflow_executor::ExecutorConfig;
  use std::path::PathBuf;
  use std::time::Duration;

  fn create_test_executor() -> Arc<WorkflowExecutor> {
    let config = ExecutorConfig {
      component_base_path: PathBuf::from("/tmp/components"),
    };
    Arc::new(WorkflowExecutor::new(config).unwrap())
  }

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

  #[tokio::test]
  async fn test_runner_creation() {
    let workflow = create_test_workflow();
    let executor = create_test_executor();
    let runner = WorkflowRunner::new(workflow, executor);

    assert_eq!(runner.workflow().workflow_id, "test-workflow");
  }

  #[tokio::test]
  async fn test_sender_cloning() {
    let workflow = create_test_workflow();
    let executor = create_test_executor();
    let runner = WorkflowRunner::new(workflow, executor);

    let sender1 = runner.sender();
    let sender2 = runner.sender();

    // Both senders should work
    assert!(!sender1.is_closed());
    assert!(!sender2.is_closed());
  }

  #[tokio::test]
  async fn test_run_sends_to_channel() {
    let workflow = create_test_workflow();
    let executor = create_test_executor();
    let mut runner = WorkflowRunner::new(workflow, executor);

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
    let workflow = create_test_workflow();
    let executor = create_test_executor();
    let runner = WorkflowRunner::new(workflow, executor);

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
