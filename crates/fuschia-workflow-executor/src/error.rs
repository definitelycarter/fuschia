//! Error types for workflow execution.

use fuschia_task_executor::TaskExecutionError;
use thiserror::Error;

/// Errors that can occur during workflow execution.
#[derive(Debug, Error)]
pub enum ExecutionError {
  /// Failed to resolve input templates for a node.
  #[error("input resolution failed for node '{node_id}': {message}")]
  InputResolution { node_id: String, message: String },

  /// Task execution failed.
  #[error("task execution failed for node '{node_id}': {source}")]
  TaskExecution {
    node_id: String,
    #[source]
    source: TaskExecutionError,
  },

  /// Workflow execution was cancelled.
  #[error("workflow execution cancelled")]
  Cancelled,

  /// Node execution timed out.
  #[error("node '{node_id}' timed out")]
  Timeout { node_id: String },

  /// Failed to load component.
  #[error("failed to load component for node '{node_id}': {message}")]
  ComponentLoad { node_id: String, message: String },

  /// Workflow graph is invalid.
  #[error("invalid workflow graph: {message}")]
  InvalidGraph { message: String },
}
