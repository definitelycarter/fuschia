use wasmtime::{Config, Engine};

/// Configuration for creating a wasmtime Engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
  /// Enable epoch-based interruption for timeouts.
  pub epoch_interruption: bool,
  /// Enable async support.
  pub async_support: bool,
}

impl Default for EngineConfig {
  fn default() -> Self {
    Self {
      epoch_interruption: true,
      async_support: true,
    }
  }
}

/// Create a wasmtime Engine with the given configuration.
///
/// The Engine should be created once and shared across all component executions
/// in a process, as it is expensive to create.
pub fn create_engine(config: EngineConfig) -> Result<Engine, wasmtime::Error> {
  let mut wasm_config = Config::new();

  wasm_config.async_support(config.async_support);
  wasm_config.epoch_interruption(config.epoch_interruption);

  // Enable the component model
  wasm_config.wasm_component_model(true);

  Engine::new(&wasm_config)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_create_engine_default() {
    // Just verify engine creation succeeds with default config
    let engine = create_engine(EngineConfig::default());
    assert!(engine.is_ok());
  }

  #[test]
  fn test_create_engine_custom() {
    let config = EngineConfig {
      epoch_interruption: false,
      async_support: false,
    };
    let engine = create_engine(config);
    assert!(engine.is_ok());
  }
}
