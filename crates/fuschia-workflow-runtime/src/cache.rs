//! Component caching for efficient wasm compilation.
//!
//! Components are compiled once and cached. Each execution gets a fresh
//! instance from the cached compiled component.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use wasmtime::Engine;
use wasmtime::component::Component;

use crate::error::RuntimeError;

/// Cache key for compiled components.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ComponentKey {
  pub name: String,
  pub version: String,
}

impl ComponentKey {
  pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      version: version.into(),
    }
  }
}

/// Caches compiled wasm components to avoid recompilation.
#[derive(Clone)]
pub struct ComponentCache {
  cache: Arc<RwLock<HashMap<ComponentKey, Component>>>,
}

impl ComponentCache {
  pub fn new() -> Self {
    Self {
      cache: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  /// Get a compiled component from cache, or compile and cache it.
  pub fn get_or_compile(
    &self,
    engine: &Engine,
    key: &ComponentKey,
    wasm_path: &Path,
  ) -> Result<Component, RuntimeError> {
    // Try read lock first
    {
      let cache = self.cache.read().unwrap();
      if let Some(component) = cache.get(key) {
        return Ok(component.clone());
      }
    }

    // Compile and insert with write lock
    let component =
      Component::from_file(engine, wasm_path).map_err(|e| RuntimeError::InvalidGraph {
        message: format!("failed to load component '{}': {}", key.name, e),
      })?;

    {
      let mut cache = self.cache.write().unwrap();
      cache.insert(key.clone(), component.clone());
    }

    Ok(component)
  }

  /// Clear the cache.
  pub fn clear(&self) {
    let mut cache = self.cache.write().unwrap();
    cache.clear();
  }
}

impl Default for ComponentCache {
  fn default() -> Self {
    Self::new()
  }
}
