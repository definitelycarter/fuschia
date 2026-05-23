/// Glue between [`LuaActor`](crate::LuaActor) and a host-specific binding set.
///
/// A `LuaHost` impl owns the globals registered into the Lua state — the
/// analog of [`WasmHost::add_to_linker`](crate::WasmHost::add_to_linker) for
/// the Wasm side, just without WIT or bindgen in the picture. The actor crate
/// provides the channel loop, JSON marshalling, and per-message Lua VM setup;
/// the host populates each fresh Lua state with whatever functions and tables
/// scripts are expected to call.
///
/// Fuchsia ships [`DefaultLuaHost`](crate::DefaultLuaHost) for the canonical
/// capability set (log + http). Hosts with additional capabilities (MQTT, BLE,
/// etc.) implement `LuaHost` themselves.
pub trait LuaHost: 'static + Send + Sync {
  /// Populate the fresh Lua state with host-provided globals. Called once
  /// per inbound message, immediately before the script source is loaded.
  fn populate(&self, lua: &mlua::Lua) -> mlua::Result<()>;
}
