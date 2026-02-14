//! Runtime error types.

use fuschia_host::HostError;

/// Errors that can occur during runtime execution.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
  /// Execution was cancelled.
  #[error("execution cancelled")]
  Cancelled,

  /// Failed to serialize component input.
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

  /// Failed to resolve node inputs (template rendering or type coercion).
  #[error("input resolution failed for node '{node_id}': {message}")]
  InputResolution { node_id: String, message: String },

  /// Failed to load/compile a wasm component.
  #[error("failed to load component for node '{node_id}': {message}")]
  ComponentLoad { node_id: String, message: String },

  /// Invalid workflow graph structure.
  #[error("invalid graph: {message}")]
  InvalidGraph { message: String },
}
