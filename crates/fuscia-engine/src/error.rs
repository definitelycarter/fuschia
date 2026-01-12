//! Error types for workflow execution.

use fuscia_host::HostError;
use thiserror::Error;

/// Errors that can occur during workflow execution.
#[derive(Debug, Error)]
pub enum ExecutionError {
  /// Failed to resolve input templates for a node.
  #[error("input resolution failed for node '{node_id}': {message}")]
  InputResolution { node_id: String, message: String },

  /// Component execution failed.
  #[error("component execution failed for node '{node_id}': {source}")]
  ComponentExecution {
    node_id: String,
    #[source]
    source: HostError,
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
