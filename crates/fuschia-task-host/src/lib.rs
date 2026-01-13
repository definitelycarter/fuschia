//! Task component execution for fuschia.
//!
//! This crate handles loading, instantiating, and executing task components.
//! It binds to the `task-component` WIT world and implements the host imports
//! (kv, config, log) by delegating to `fuschia-host::HostState`.

mod bindings;
mod executor;

pub use bindings::{Context, Output, TaskHostState};
pub use executor::{TaskResult, execute_task, load_component};
