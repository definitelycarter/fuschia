use crate::host::WasmHost;
use async_trait::async_trait;
use fuchsia_actor::{Actor, ActorError, Context, Emitter, Inbox};
use serde_json::Value;
use std::sync::Arc;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

/// A [`fuchsia_actor::Actor`] backed by a wasm component.
///
/// `WasmActor` is generic over a [`WasmHost`] that supplies the world-specific
/// concerns (state shape, bindings type, import wiring, execute trampoline).
/// Fuchsia ships [`DefaultHost`](crate::DefaultHost) for the canonical
/// `actor-component` world; hosts with custom capabilities define their own.
///
/// Per message: a fresh `Store<H::State>` is created, the component is
/// instantiated against the pre-built `Linker`, `task.execute` is invoked,
/// and the resulting JSON is emitted to downstream nodes.
///
/// Cheap to clone — `engine`, `component`, `linker`, and `host` are all
/// `Arc`-backed (or have Arc semantics in their respective types).
pub struct WasmActor<H: WasmHost> {
  pub(crate) engine: Engine,
  pub(crate) component: Component,
  pub(crate) linker: Arc<Linker<H::State>>,
  pub(crate) host: Arc<H>,
  pub(crate) epoch_deadline: u64,
}

impl<H: WasmHost> Clone for WasmActor<H> {
  fn clone(&self) -> Self {
    Self {
      engine: self.engine.clone(),
      component: self.component.clone(),
      linker: Arc::clone(&self.linker),
      host: Arc::clone(&self.host),
      epoch_deadline: self.epoch_deadline,
    }
  }
}

impl<H: WasmHost> WasmActor<H> {
  /// Start building a `WasmActor` with the given engine and host.
  pub fn builder(engine: Engine, host: H) -> crate::WasmActorBuilder<H> {
    crate::WasmActorBuilder::new(engine, host)
  }

  async fn invoke(&self, input: Value, ctx: &Context) -> Result<Value, ActorError> {
    let mut store = Store::new(&self.engine, self.host.fresh_state());
    store.set_epoch_deadline(self.epoch_deadline);

    let bindings = self
      .host
      .instantiate(&mut store, &self.component, &self.linker)
      .await
      .map_err(|e| ActorError::Other(format!("wasm instantiation failed: {e}")))?;

    let data = serde_json::to_string(&input)
      .map_err(|e| ActorError::Other(format!("input serialize: {e}")))?;

    let result = self
      .host
      .execute(&bindings, &mut store, ctx, &data)
      .await
      .map_err(|e| ActorError::Other(format!("wasm trap: {e}")))?;

    match result {
      Ok(output) => {
        serde_json::from_str(&output).map_err(|e| ActorError::Other(format!("output parse: {e}")))
      }
      Err(msg) => Err(ActorError::Other(format!(
        "component reported error: {msg}"
      ))),
    }
  }
}

#[async_trait]
impl<H: WasmHost> Actor for WasmActor<H> {
  async fn run(&self, mut inbox: Inbox, emit: Emitter, ctx: Context) -> Result<(), ActorError> {
    loop {
      tokio::select! {
          _ = ctx.cancelled() => return Ok(()),
          msg = inbox.recv() => match msg {
              Some(v) => {
                  let output = self.invoke(v, &ctx).await?;
                  emit.send(output).await?;
              }
              None => return Ok(()),
          }
      }
    }
  }
}
