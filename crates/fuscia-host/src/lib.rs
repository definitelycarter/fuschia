//! Shared wasmtime infrastructure for fuscia component hosting.
//!
//! This crate provides:
//! - Engine configuration with epoch-based interruption
//! - Store setup helpers and host state
//! - Pluggable KV store trait with in-memory implementation
//! - Common error types for component execution
//!
//! It does not know about tasks or triggers specifically - those are handled
//! by `fuscia-task-host` and `fuscia-trigger-host` which build on this foundation.

mod engine;
mod error;
mod kv;
mod state;

pub use engine::{EngineConfig, create_engine};
pub use error::HostError;
pub use kv::{InMemoryKvStore, KvStore};
pub use state::{HostState, create_store};
