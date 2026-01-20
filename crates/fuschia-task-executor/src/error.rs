//! Task execution errors.

use fuschia_host::HostError;

/// Errors that can occur during task execution.
#[derive(Debug, thiserror::Error)]
pub enum TaskExecutionError {
  /// Task execution was cancelled.
  #[error("task cancelled")]
  Cancelled,

  /// Failed to serialize task input.
  #[error("failed to serialize input: {message}")]
  InputSerialization { message: String },

  /// Component execution failed.
  #[error("component execution failed: {source}")]
  ComponentExecution {
    #[source]
    source: HostError,
  },

  /// Failed to parse component output.
  #[error("invalid component output: {message}")]
  InvalidOutput { message: String },

  /// Task type not supported.
  #[error("unsupported task type: {message}")]
  UnsupportedTaskType { message: String },
}
