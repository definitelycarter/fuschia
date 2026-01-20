//! Fuschia Workflow Engine
//!
//! This crate provides the workflow execution engine for fuschia.
//! It re-exports types from `fuschia-workflow-executor` and provides
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
//! │                    WorkflowExecutor                         │
//! │  - execute(workflow, payload, cancel) → ExecutionResult     │
//! │  - graph traversal, scheduling                              │
//! │  - input resolution via minijinja                           │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      TaskExecutor                           │
//! │  - executes individual tasks via wasm runtime               │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use fuschia_engine::{WorkflowExecutor, WorkflowRunner, ExecutorConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! // Create the executor
//! let config = ExecutorConfig {
//!     component_base_path: PathBuf::from("/path/to/components"),
//! };
//! let executor = Arc::new(WorkflowExecutor::new(config)?);
//!
//! // Create a runner for a workflow
//! let runner = WorkflowRunner::new(workflow, executor);
//!
//! // Get sender for external triggers
//! let sender = runner.sender();
//!
//! // Start the execution loop
//! let cancel = CancellationToken::new();
//! runner.start(cancel).await?;
//! ```

mod runner;

// Re-export from fuschia-workflow-executor
pub use fuschia_workflow_executor::{
  ExecutionError, ExecutionResult, ExecutorConfig, TaskResult, WorkflowExecutor,
};

pub use runner::WorkflowRunner;
