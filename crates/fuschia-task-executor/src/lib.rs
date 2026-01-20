//! Task execution for fuschia workflows.
//!
//! This crate provides the [`TaskExecutor`] which handles task setup,
//! execution, and teardown. It takes prepared task data and executes
//! it against the appropriate runtime (wasm component, HTTP, etc.).

mod error;
mod executor;
mod result;

pub use error::TaskExecutionError;
pub use executor::{TaskExecutor, TaskInput};
pub use result::TaskResult;
