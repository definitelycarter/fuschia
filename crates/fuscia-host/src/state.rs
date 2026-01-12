use std::collections::HashMap;
use wasmtime::component::ResourceTable;
use wasmtime::{Engine, Store};

use crate::kv::KvStore;

/// Host state stored in the wasmtime Store.
///
/// This holds all the state needed to implement host imports (kv, config, log).
/// Each component execution gets a fresh HostState.
pub struct HostState<K: KvStore> {
  /// Resource table for managing WIT resources.
  pub table: ResourceTable,

  /// Execution-scoped key-value store.
  pub kv: K,

  /// Component configuration (from workflow node inputs).
  config: HashMap<String, String>,

  /// Execution context for logging.
  pub execution_id: String,
  pub node_id: String,
}

impl<K: KvStore> HostState<K> {
  /// Create a new host state for a component execution.
  pub fn new(execution_id: String, node_id: String, kv: K) -> Self {
    Self {
      table: ResourceTable::new(),
      kv,
      config: HashMap::new(),
      execution_id,
      node_id,
    }
  }

  /// Set configuration values (called before component execution).
  pub fn set_config(&mut self, config: HashMap<String, String>) {
    self.config = config;
  }

  /// Get a configuration value.
  pub fn config_get(&self, key: &str) -> Option<&String> {
    self.config.get(key)
  }
}

/// Create a new Store with HostState for component execution.
///
/// If `epoch_deadline` is provided, sets the epoch deadline for timeout enforcement.
/// The caller should spawn a task to increment the engine's epoch after the timeout.
pub fn create_store<K: KvStore>(
  engine: &Engine,
  state: HostState<K>,
  epoch_deadline: Option<u64>,
) -> Store<HostState<K>> {
  let mut store = Store::new(engine, state);

  if let Some(deadline) = epoch_deadline {
    store.set_epoch_deadline(deadline);
  }

  store
}
