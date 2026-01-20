//! Workflow runtime for fuschia.
//!
//! This crate provides the core runtime for executing workflows and individual nodes.
//!
//! # Architecture
//!
//! ```text
//! WorkflowRuntime
//! ├── new(config, workflow) - creates runtime with resolved workflow
//! ├── execute_workflow(payload) -> WorkflowExecution
//! └── execute_node(node_id, input) -> TaskExecution
//!
//! WorkflowExecution
//! └── wait() - graph traversal, calls execute_node for each ready node
//!
//! TaskExecution
//! └── wait() - input resolution + wasm execution
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use fuschia_workflow_runtime::{RuntimeConfig, WorkflowRuntime};
//!
//! let config = RuntimeConfig {
//!     component_base_path: PathBuf::from("/path/to/components"),
//! };
//! let runtime = WorkflowRuntime::new(config, workflow)?;
//!
//! // Execute entire workflow
//! let result = runtime.execute_workflow(payload, cancel).wait().await?;
//!
//! // Execute single node
//! let result = runtime.execute_node("my_node", input, cancel)?.wait().await?;
//! ```

mod cache;
mod error;
mod execution;
mod input;
mod runtime;
mod task;

pub use error::RuntimeError;
pub use execution::{WorkflowExecution, WorkflowResult};
pub use runtime::{RuntimeConfig, WorkflowRuntime};
pub use task::{TaskExecution, TaskResult};
