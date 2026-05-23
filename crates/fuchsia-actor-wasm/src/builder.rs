use crate::actor::WasmActor;
use crate::host::WasmHost;
use fuchsia_actor::ActorError;
use std::path::PathBuf;
use std::sync::Arc;
use wasmtime::Engine;
use wasmtime::component::{Component, Linker};

/// Builds a [`WasmActor`]. Acquired via [`WasmActor::builder`].
///
/// A component source is required (one of `component`, `component_from_path`,
/// or `component_from_bytes`). Other settings are optional.
pub struct WasmActorBuilder<H: WasmHost> {
  engine: Engine,
  host: H,
  component: Option<ComponentSource>,
  epoch_deadline: u64,
}

enum ComponentSource {
  Compiled(Component),
  Path(PathBuf),
  Bytes(Vec<u8>),
}

impl<H: WasmHost> WasmActorBuilder<H> {
  pub(crate) fn new(engine: Engine, host: H) -> Self {
    Self {
      engine,
      host,
      component: None,
      epoch_deadline: u64::MAX,
    }
  }

  /// Use an already-compiled `Component`. Best when the host compiles once
  /// and shares the component across many actor instances.
  pub fn component(mut self, component: Component) -> Self {
    self.component = Some(ComponentSource::Compiled(component));
    self
  }

  /// Load and compile a component from a `.wasm` file on disk.
  pub fn component_from_path(mut self, path: impl Into<PathBuf>) -> Self {
    self.component = Some(ComponentSource::Path(path.into()));
    self
  }

  /// Compile a component from in-memory bytes.
  pub fn component_from_bytes(mut self, bytes: Vec<u8>) -> Self {
    self.component = Some(ComponentSource::Bytes(bytes));
    self
  }

  /// Epoch deadline (in ticks) applied to each fresh `Store`. Defaults to
  /// `u64::MAX` (effectively no deadline). The host is responsible for
  /// driving the engine's epoch ticker if timeouts should fire.
  pub fn epoch_deadline(mut self, ticks: u64) -> Self {
    self.epoch_deadline = ticks;
    self
  }

  pub fn build(self) -> Result<WasmActor<H>, ActorError> {
    let component = match self.component {
      Some(ComponentSource::Compiled(c)) => c,
      Some(ComponentSource::Path(p)) => Component::from_file(&self.engine, &p)
        .map_err(|e| ActorError::Other(format!("compile component from {}: {e}", p.display())))?,
      Some(ComponentSource::Bytes(b)) => Component::new(&self.engine, &b)
        .map_err(|e| ActorError::Other(format!("compile component from bytes: {e}")))?,
      None => {
        return Err(ActorError::Other(
          "WasmActorBuilder requires a component (component, component_from_path, or component_from_bytes)".into(),
        ));
      }
    };

    let mut linker = Linker::<H::State>::new(&self.engine);
    self
      .host
      .add_to_linker(&mut linker)
      .map_err(|e| ActorError::Other(format!("link host imports: {e}")))?;

    Ok(WasmActor {
      engine: self.engine,
      component,
      linker: Arc::new(linker),
      host: Arc::new(self.host),
      epoch_deadline: self.epoch_deadline,
    })
  }
}
