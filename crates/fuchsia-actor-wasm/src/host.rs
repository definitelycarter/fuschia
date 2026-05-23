use async_trait::async_trait;
use fuchsia_actor::Context;
use wasmtime::Store;
use wasmtime::component::{Component, Linker};
use wasmtime_wasi::WasiView;

/// Glue between [`WasmActor`](crate::WasmActor) and a host-specific wasm world.
///
/// A `WasmHost` impl owns the world-specific concerns — generated bindings,
/// per-Store state, linker wiring, and the bridge between WIT host functions
/// and host-side capability handles. The actor crate provides the channel
/// loop, JSON marshalling, and lifecycle; the host fills in the world-specific
/// gaps.
///
/// Fuchsia ships [`DefaultHost`](crate::DefaultHost) for the canonical
/// `actor-component` world (log + http). Hosts with additional capabilities
/// (MQTT, BLE, etc.) define their own WIT world and implement `WasmHost`
/// for the corresponding bindgen output.
#[async_trait]
pub trait WasmHost: 'static + Send + Sync {
  /// Per-Store state. Holds the `WasiCtx`, capability handles, and any
  /// per-instance bookkeeping. Created fresh by [`fresh_state`] for every
  /// message — wasm memory is never shared across invocations.
  ///
  /// [`fresh_state`]: WasmHost::fresh_state
  type State: WasiView + 'static + Send;

  /// Typed bindings returned by `bindgen!`. The exact type is opaque to the
  /// actor crate; the host produces it in [`instantiate`] and consumes it
  /// in [`execute`].
  ///
  /// [`instantiate`]: WasmHost::instantiate
  /// [`execute`]: WasmHost::execute
  type Bindings: Send;

  /// Wire host functions into the linker so the component's imports are
  /// satisfied. Called once at builder time.
  fn add_to_linker(&self, linker: &mut Linker<Self::State>) -> wasmtime::Result<()>;

  /// Build per-Store state. Called fresh for every inbound message.
  fn fresh_state(&self) -> Self::State;

  /// Instantiate the component into the store using the (pre-built) linker.
  /// The returned bindings are passed back into [`execute`].
  ///
  /// [`execute`]: WasmHost::execute
  async fn instantiate(
    &self,
    store: &mut Store<Self::State>,
    component: &Component,
    linker: &Linker<Self::State>,
  ) -> wasmtime::Result<Self::Bindings>;

  /// Invoke the component's `task.execute` export. `ctx` carries the actor's
  /// per-message context (node id, etc.); `input` is the JSON-serialized
  /// payload. Returns either a JSON-serialized success output or the
  /// component-reported error string.
  async fn execute(
    &self,
    bindings: &Self::Bindings,
    store: &mut Store<Self::State>,
    ctx: &Context,
    input: &str,
  ) -> wasmtime::Result<Result<String, String>>;
}
