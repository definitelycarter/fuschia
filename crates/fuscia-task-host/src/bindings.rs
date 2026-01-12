//! WIT bindings for the task-component world.

use fuscia_host::{HostState, InMemoryKvStore, KvStore};
use wasmtime::component::HasData;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Host state wrapper for task component execution.
/// This wraps HostState to implement HasData in this crate.
pub struct TaskHostState {
  pub inner: HostState<InMemoryKvStore>,
  pub wasi: WasiCtx,
  pub table: ResourceTable,
}

impl TaskHostState {
  pub fn new(execution_id: String, node_id: String) -> Self {
    let wasi = WasiCtxBuilder::new().build();
    Self {
      inner: HostState::new(execution_id, node_id, InMemoryKvStore::new()),
      wasi,
      table: ResourceTable::new(),
    }
  }
}

impl WasiView for TaskHostState {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.wasi,
      table: &mut self.table,
    }
  }
}

// Implement HasData so we can use add_to_linker
impl HasData for TaskHostState {
  type Data<'a> = &'a mut Self;
}

// Generate bindings for task-component world
wasmtime::component::bindgen!({
    path: "../../wit",
    world: "fuscia:platform/task-component@0.1.0",
    exports: { default: async },
});

/// Implement the host imports for task components.
impl fuscia::kv::kv::Host for TaskHostState {
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

impl fuscia::config::config::Host for TaskHostState {
  fn get(&mut self, key: String) -> Option<String> {
    self.inner.config_get(&key).cloned()
  }
}

impl fuscia::log::log::Host for TaskHostState {
  fn log(&mut self, level: fuscia::log::log::Level, message: String) {
    use fuscia::log::log::Level;
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
pub use exports::fuscia::task::task::{Context, Output};
