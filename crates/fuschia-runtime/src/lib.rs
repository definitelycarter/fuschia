//! Fuschia Runtime
//!
//! This crate provides the workflow runtime for fuschia. It handles
//! component execution, graph traversal, and workflow orchestration.
//!
//! The lowest-level primitives are [`TaskExecutor`] for task wasm components
//! and (future) `TriggerExecutor` for trigger wasm components. Higher-level
//! constructs like `Runtime` (coming in future steps) will use these to
//! execute full workflows.

mod cache;
mod error;
mod input;
mod result;
mod runtime;
mod task;
mod trigger;

pub use cache::{ComponentCache, ComponentKey};
pub use error::RuntimeError;
pub use input::{SchemaType, coerce_inputs, extract_schema_types, resolve_inputs};
pub use result::{InvokeResult, NodeResult};
pub use runtime::{Runtime, RuntimeConfig};
pub use task::{TaskExecutor, TaskInput};
pub use trigger::{TriggerExecutor, TriggerInput};
