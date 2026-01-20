//! Workflow execution for fuschia.
//!
//! This crate provides the [`WorkflowExecutor`] which handles:
//! - Graph traversal and node scheduling
//! - Template resolution via minijinja
//! - Parallel task execution
//! - Component caching
//!
//! The executor takes a notifier that implements both [`WorkflowNotifier`] and
//! [`TaskNotifier`] to ensure consistent event handling across workflow and task levels.

mod cache;
mod error;
mod executor;
mod input;
mod result;

pub use cache::{ComponentCache, ComponentKey};
pub use error::ExecutionError;
pub use executor::{ExecutorConfig, WorkflowExecutor};
pub use input::{SchemaType, coerce_inputs, extract_schema_types, resolve_inputs};
pub use result::{ExecutionResult, TaskResult};
