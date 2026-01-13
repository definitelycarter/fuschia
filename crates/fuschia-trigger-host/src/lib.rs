//! Trigger component execution for fuschia.
//!
//! This crate handles loading, instantiating, and executing trigger components.
//! It binds to the `trigger-component` WIT world and implements the host imports
//! (kv, config, log) by delegating to `fuschia-host::HostState`.

mod bindings;
mod executor;

pub use bindings::{Event, Status, TriggerHostState, WebhookRequest};
pub use executor::{TriggerResult, execute_trigger, load_component};
