//! Runtime errors.

use fuschia_host::HostError;

/// Errors that can occur during runtime operations.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
  /// Node not found in workflow.
  #[error("node '{node_id}' not found in workflow")]
  NodeNotFound { node_id: String },

  /// Invalid workflow graph.
  #[error("invalid workflow graph: {message}")]
  InvalidGraph { message: String },

  /// Input resolution failed.
  #[error("failed to resolve inputs for node '{node_id}': {message}")]
  InputResolution { node_id: String, message: String },

  /// Component execution failed.
  #[error("component execution failed for node '{node_id}'")]
  ComponentExecution {
    node_id: String,
    #[source]
    source: HostError,
  },

  /// Execution was cancelled.
  #[error("execution cancelled")]
  Cancelled,
}
