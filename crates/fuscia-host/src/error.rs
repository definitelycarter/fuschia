use thiserror::Error;
use wasmtime::component::ResourceTableError;

/// Errors that can occur during component execution.
#[derive(Debug, Error)]
pub enum HostError {
  /// Component instantiation failed (bad wasm, missing imports, etc.)
  #[error("instantiation failed: {message}")]
  Instantiation { message: String },

  /// Component trapped or encountered a runtime error.
  #[error("wasmtime error: {0}")]
  Wasmtime(#[from] wasmtime::Error),

  /// Component returned an error via WIT result.
  /// This is not necessarily a task failure - the engine decides based on workflow config.
  #[error("component error: {message}")]
  ComponentError { message: String },

  /// Resource table error (e.g., invalid handle).
  #[error("resource error: {0}")]
  Resource(#[from] ResourceTableError),
}

impl HostError {
  /// Create an instantiation error.
  pub fn instantiation(message: impl Into<String>) -> Self {
    Self::Instantiation {
      message: message.into(),
    }
  }

  /// Create a component error (returned via WIT result, not a trap).
  pub fn component_error(message: impl Into<String>) -> Self {
    Self::ComponentError {
      message: message.into(),
    }
  }
}
