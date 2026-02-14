//! Fuschia Workflow Engine
//!
//! This crate provides the workflow execution engine for fuschia.
//! It re-exports types from `fuschia-runtime` and provides
//! a `WorkflowRunner` for channel-based workflow triggering.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      WorkflowRunner                         │
//! │  - owns mpsc channel (sender + receiver)                    │
//! │  - run(payload) triggers execution                          │
//! │  - start(cancel) runs the execution loop                    │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Runtime                              │
//! │  - invoke(payload, cancel) → InvokeResult                   │
//! │  - invoke_node(node_id, payload, cancel) → NodeResult       │
//! │  - graph traversal, scheduling                              │
//! │  - input resolution via minijinja                           │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    TaskExecutor / TriggerExecutor            │
//! │  - executes individual components via wasm runtime           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use fuschia_engine::WorkflowRunner;
//! use fuschia_runtime::{Runtime, RuntimeConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! // Create the runtime
//! let config = RuntimeConfig {
//!     component_base_path: PathBuf::from("/path/to/components"),
//! };
//! let runtime = Arc::new(Runtime::new(workflow, config)?);
//!
//! // Create a runner
//! let runner = WorkflowRunner::new(runtime);
//!
//! // Get sender for external triggers
//! let sender = runner.sender();
//!
//! // Start the execution loop
//! let cancel = CancellationToken::new();
//! runner.start(cancel).await?;
//! ```

mod runner;

// Re-export from fuschia-runtime
pub use fuschia_runtime::{InvokeResult, NodeResult, Runtime, RuntimeConfig, RuntimeError};

pub use runner::WorkflowRunner;
