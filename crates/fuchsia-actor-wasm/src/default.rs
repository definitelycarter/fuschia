//! `DefaultHost` — built-in [`WasmHost`] implementation for the canonical
//! `fuchsia:platform/actor-component` world (log + http + emit imports,
//! actor lifecycle export).

use crate::host::WasmHost;
use async_trait::async_trait;
use fuchsia_actor::{Context, Emitter};
use fuchsia_capabilities::http::{HttpClient, HttpError, HttpRequest, HttpResponse};
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::Store;
use wasmtime::component::{Component, HasData, Linker};
use wasmtime_wasi::p2::add_to_linker_async;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "../../wit",
    world: "fuchsia:platform/actor-component@0.1.0",
    imports: { default: async },
    exports: { default: async },
});

use exports::fuchsia::actor::actor::Context as WitContext;

/// Per-`Store` state for [`DefaultHost`]. Holds the `WasiCtx`, the HTTP
/// client, and the downstream `Emitter` so the `emit` import callback can
/// reach it. Built once per actor instance in [`DefaultHost::initial_state`].
pub struct DefaultHostState {
  wasi: WasiCtx,
  table: ResourceTable,
  http: Arc<dyn HttpClient>,
  emitter: Emitter,
}

impl WasiView for DefaultHostState {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.wasi,
      table: &mut self.table,
    }
  }
}

impl HasData for DefaultHostState {
  type Data<'a> = &'a mut Self;
}

// ---- log import: route component log calls through tracing ---------------
//
// The actor's tokio task is `.instrument()`-ed with a span containing the
// node id, so events emitted here automatically inherit that context.

impl fuchsia::log::log::Host for DefaultHostState {
  async fn log(&mut self, level: fuchsia::log::log::Level, message: String) {
    use fuchsia::log::log::Level::*;
    match level {
      Trace => tracing::trace!(target: "wasm.component", "{message}"),
      Debug => tracing::debug!(target: "wasm.component", "{message}"),
      Info => tracing::info!(target: "wasm.component", "{message}"),
      Warn => tracing::warn!(target: "wasm.component", "{message}"),
      Error => tracing::error!(target: "wasm.component", "{message}"),
    }
  }
}

// ---- http import: delegate to injected HttpClient -------------------------

impl fuchsia::http::outbound::Host for DefaultHostState {
  async fn send(
    &mut self,
    req: fuchsia::http::outbound::HttpRequest,
  ) -> Result<fuchsia::http::outbound::HttpResponse, fuchsia::http::outbound::HttpError> {
    let request = HttpRequest {
      method: req.method,
      url: req.url,
      headers: req.headers.into_iter().collect::<HashMap<_, _>>(),
      body: req.body,
    };

    Arc::clone(&self.http)
      .send(request)
      .await
      .map(|resp: HttpResponse| fuchsia::http::outbound::HttpResponse {
        status: resp.status,
        headers: resp.headers.into_iter().collect(),
        body: resp.body,
      })
      .map_err(|e: HttpError| match e {
        HttpError::HostNotAllowed { host } => {
          fuchsia::http::outbound::HttpError::HostNotAllowed(host)
        }
        HttpError::RequestFailed(msg) => fuchsia::http::outbound::HttpError::RequestFailed(msg),
        HttpError::InvalidUrl(msg) => fuchsia::http::outbound::HttpError::InvalidUrl(msg),
      })
  }
}

// ---- emit import: forward component emissions to the downstream channel ---

impl fuchsia::actor::emit::Host for DefaultHostState {
  async fn send(&mut self, data: String) -> Result<(), String> {
    let value = serde_json::from_str(&data).map_err(|e| format!("emit: invalid JSON: {e}"))?;
    self
      .emitter
      .send(value)
      .await
      .map_err(|_| "channel closed".to_string())
  }
}

/// Built-in [`WasmHost`] for the canonical `actor-component` world.
///
/// Wires `log` (→ `tracing`), `http` (→ the injected `HttpClient`), and
/// `emit` (→ the actor's downstream channel). Hosts that only need these
/// three capabilities can register their wasm actors with
/// `WasmActor<DefaultHost>` directly; richer hosts implement `WasmHost`
/// themselves.
#[derive(Clone)]
pub struct DefaultHost {
  http: Arc<dyn HttpClient>,
}

impl DefaultHost {
  pub fn new(http: Arc<dyn HttpClient>) -> Self {
    Self { http }
  }
}

#[async_trait]
impl WasmHost for DefaultHost {
  type State = DefaultHostState;
  type Bindings = ActorComponent;

  fn add_to_linker(&self, linker: &mut Linker<Self::State>) -> wasmtime::Result<()> {
    add_to_linker_async(linker)?;
    ActorComponent::add_to_linker::<DefaultHostState, DefaultHostState>(linker, |s| s)?;
    Ok(())
  }

  fn initial_state(&self, emitter: Emitter) -> Self::State {
    DefaultHostState {
      wasi: WasiCtxBuilder::new().build(),
      table: ResourceTable::new(),
      http: Arc::clone(&self.http),
      emitter,
    }
  }

  async fn instantiate(
    &self,
    store: &mut Store<Self::State>,
    component: &Component,
    linker: &Linker<Self::State>,
  ) -> wasmtime::Result<Self::Bindings> {
    ActorComponent::instantiate_async(store, component, linker).await
  }

  async fn call_setup(
    &self,
    bindings: &Self::Bindings,
    store: &mut Store<Self::State>,
    ctx: &Context,
  ) -> wasmtime::Result<Result<(), String>> {
    let wit_ctx = wit_context(ctx);
    bindings
      .fuchsia_actor_actor()
      .call_setup(store, &wit_ctx)
      .await
  }

  async fn call_handle(
    &self,
    bindings: &Self::Bindings,
    store: &mut Store<Self::State>,
    ctx: &Context,
    message: &str,
  ) -> wasmtime::Result<Result<(), String>> {
    let wit_ctx = wit_context(ctx);
    bindings
      .fuchsia_actor_actor()
      .call_handle(store, &wit_ctx, message)
      .await
  }

  async fn call_teardown(
    &self,
    bindings: &Self::Bindings,
    store: &mut Store<Self::State>,
    ctx: &Context,
  ) -> wasmtime::Result<Result<(), String>> {
    let wit_ctx = wit_context(ctx);
    bindings
      .fuchsia_actor_actor()
      .call_teardown(store, &wit_ctx)
      .await
  }
}

fn wit_context(ctx: &Context) -> WitContext {
  WitContext {
    execution_id: String::new(),
    node_id: ctx.node_id.clone(),
    task_id: String::new(),
  }
}
