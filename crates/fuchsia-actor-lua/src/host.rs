use fuchsia_actor::Emitter;

/// Glue between [`LuaActor`](crate::LuaActor) and a host-specific binding set.
///
/// A `LuaHost` impl owns the globals registered into the Lua state — the
/// analog of [`WasmHost::add_to_linker`](crate::WasmHost::add_to_linker) for
/// the Wasm side, just without WIT or bindgen in the picture. The actor crate
/// provides the channel loop and lifecycle orchestration; the host populates
/// the Lua state with whatever globals scripts are expected to call.
///
/// Implementations must register an `emit(data)` global that forwards into
/// the provided [`Emitter`] — that's how scripts push downstream payloads.
///
/// Fuchsia ships [`DefaultLuaHost`](crate::DefaultLuaHost) for the canonical
/// capability set (log + http + emit). Hosts with additional capabilities
/// (MQTT, BLE, etc.) implement `LuaHost` themselves.
pub trait LuaHost: 'static + Send + Sync {
  /// Populate the Lua state with host-provided globals. Called once when
  /// the actor starts running, before the script source is loaded. The
  /// provided `Emitter` is the actor's outbound channel — implementations
  /// must wire it into an `emit` global the script can call.
  fn populate(&self, lua: &mlua::Lua, emitter: Emitter) -> mlua::Result<()>;
}
