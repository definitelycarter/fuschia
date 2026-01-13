use thiserror::Error;

/// Errors that can occur during task execution.
#[derive(Debug, Error)]
pub enum TaskError {
  /// Missing required input field.
  #[error("missing required input: {field}")]
  MissingInput { field: String },

  /// Invalid input value.
  #[error("invalid input '{field}': {message}")]
  InvalidInput { field: String, message: String },

  /// HTTP request failed.
  #[error("http error: {0}")]
  Http(#[from] reqwest::Error),

  /// Component execution failed.
  #[error("component error: {message}")]
  Component { message: String },

  /// Task timed out.
  #[error("task timed out after {timeout_ms}ms")]
  Timeout { timeout_ms: u64 },
}
