//! Lua-script-hosting [`Actor`] implementation for `fuchsia-runtime`.
//!
//! The actor is generic over [`LuaHost`] — the trait that owns the
//! globals registered into the Lua state. Fuchsia ships [`DefaultLuaHost`]
//! for the canonical capability set (log + http). Hosts with additional
//! capabilities (MQTT, BLE, etc.) implement `LuaHost` themselves and
//! register their own Lua globals.
//!
//! [`Actor`]: fuchsia_actor::Actor

mod actor;
mod builder;
mod default;
mod host;

pub use actor::LuaActor;
pub use builder::LuaActorBuilder;
pub use default::DefaultLuaHost;
pub use host::LuaHost;
