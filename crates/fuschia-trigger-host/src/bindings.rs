//! WIT bindings for the trigger-component world.

use fuschia_host::{HostState, InMemoryKvStore, KvStore};
use wasmtime::component::HasData;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Host state wrapper for trigger component execution.
/// This wraps HostState to implement HasData in this crate.
pub struct TriggerHostState {
  pub inner: HostState<InMemoryKvStore>,
  pub wasi: WasiCtx,
  pub table: ResourceTable,
}

impl TriggerHostState {
  pub fn new(execution_id: String, node_id: String) -> Self {
    let wasi = WasiCtxBuilder::new().build();
    Self {
      inner: HostState::new(execution_id, node_id, InMemoryKvStore::new()),
      wasi,
      table: ResourceTable::new(),
    }
  }
}

impl WasiView for TriggerHostState {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.wasi,
      table: &mut self.table,
    }
  }
}

// Implement HasData so we can use add_to_linker
impl HasData for TriggerHostState {
  type Data<'a> = &'a mut Self;
}

// Generate bindings for trigger-component world
wasmtime::component::bindgen!({
    path: "../../wit",
    world: "fuschia:platform/trigger-component@0.1.0",
    exports: { default: async },
});

/// Implement the host imports for trigger components.
impl fuschia::kv::kv::Host for TriggerHostState {
  fn get(&mut self, key: String) -> Option<String> {
    futures::executor::block_on(self.inner.kv.get(&key))
  }

  fn set(&mut self, key: String, value: String) {
    futures::executor::block_on(self.inner.kv.set(&key, value))
  }

  fn delete(&mut self, key: String) {
    futures::executor::block_on(self.inner.kv.delete(&key))
  }
}

impl fuschia::config::config::Host for TriggerHostState {
  fn get(&mut self, key: String) -> Option<String> {
    self.inner.config_get(&key).cloned()
  }
}

impl fuschia::log::log::Host for TriggerHostState {
  fn log(&mut self, level: fuschia::log::log::Level, message: String) {
    use fuschia::log::log::Level;
    match level {
      Level::Trace => {
        tracing::trace!(execution_id = %self.inner.execution_id, node_id = %self.inner.node_id, "{}", message)
      }
      Level::Debug => {
        tracing::debug!(execution_id = %self.inner.execution_id, node_id = %self.inner.node_id, "{}", message)
      }
      Level::Info => {
        tracing::info!(execution_id = %self.inner.execution_id, node_id = %self.inner.node_id, "{}", message)
      }
      Level::Warn => {
        tracing::warn!(execution_id = %self.inner.execution_id, node_id = %self.inner.node_id, "{}", message)
      }
      Level::Error => {
        tracing::error!(execution_id = %self.inner.execution_id, node_id = %self.inner.node_id, "{}", message)
      }
    }
  }
}

// Re-export generated types
pub use exports::fuschia::trigger::trigger::{Event, Status, WebhookRequest};
