//! `DefaultHost` — built-in [`WasmHost`] implementation for the canonical
//! `fuchsia:platform/actor-component` world (log + http imports, task export).

use crate::host::WasmHost;
use async_trait::async_trait;
use fuchsia_actor::Context;
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
    exports: { default: async },
});

use exports::fuchsia::task::task::Context as WitContext;

/// Per-`Store` state for [`DefaultHost`]. Holds the `WasiCtx` and a clone of
/// the host's HTTP client so import host functions can route through it.
pub struct DefaultHostState {
  wasi: WasiCtx,
  table: ResourceTable,
  http: Arc<dyn HttpClient>,
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
  fn log(&mut self, level: fuchsia::log::log::Level, message: String) {
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
//
// WIT functions are sync; the HttpClient trait is async. We bridge with
// `futures::executor::block_on` on a separate executor so it doesn't deadlock
// the wasmtime worker. The wart goes away when we move WIT imports to async.

impl fuchsia::http::outbound::Host for DefaultHostState {
  fn send(
    &mut self,
    req: fuchsia::http::outbound::HttpRequest,
  ) -> Result<fuchsia::http::outbound::HttpResponse, fuchsia::http::outbound::HttpError> {
    let request = HttpRequest {
      method: req.method,
      url: req.url,
      headers: req.headers.into_iter().collect::<HashMap<_, _>>(),
      body: req.body,
    };

    let http = Arc::clone(&self.http);
    let result = futures::executor::block_on(http.send(request));

    result
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

/// Built-in [`WasmHost`] for the canonical `actor-component` world.
///
/// Wires `log` (→ `tracing`) and `http` (→ the injected `HttpClient`).
/// Hosts that only need these two capabilities can register their wasm
/// actors with `WasmActor<DefaultHost>` directly; richer hosts implement
/// `WasmHost` themselves.
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

  fn fresh_state(&self) -> Self::State {
    DefaultHostState {
      wasi: WasiCtxBuilder::new().build(),
      table: ResourceTable::new(),
      http: Arc::clone(&self.http),
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

  async fn execute(
    &self,
    bindings: &Self::Bindings,
    store: &mut Store<Self::State>,
    ctx: &Context,
    input: &str,
  ) -> wasmtime::Result<Result<String, String>> {
    let wit_ctx = WitContext {
      execution_id: String::new(),
      node_id: ctx.node_id.clone(),
      task_id: String::new(),
    };
    let result = bindings
      .fuchsia_task_task()
      .call_execute(store, &wit_ctx, input)
      .await?;
    Ok(result.map(|o| o.data))
  }
}
