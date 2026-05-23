//! Wasm component-hosting [`Actor`] implementation for `fuchsia-runtime`.
//!
//! The actor is generic over [`WasmHost`] — the trait that owns the
//! world-specific concerns (bindgen output, per-store state, import wiring).
//! Fuchsia ships [`DefaultHost`] for the canonical `actor-component` world
//! (log + http). Hosts with custom capabilities (MQTT, BLE, etc.) define
//! their own WIT world and implement `WasmHost` for the bindgen output.
//!
//! [`Actor`]: fuchsia_actor::Actor

mod actor;
mod builder;
mod default;
mod host;

pub use actor::WasmActor;
pub use builder::WasmActorBuilder;
pub use default::{DefaultHost, DefaultHostState};
pub use host::WasmHost;
