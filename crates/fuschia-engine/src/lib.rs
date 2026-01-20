//! Fuschia Workflow Engine
//!
//! This crate provides the workflow execution engine for fuschia.
//! It re-exports types from `fuschia-workflow-runtime` and provides
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
//! │                     WorkflowRuntime                         │
//! │  - execute_workflow(payload, cancel) → WorkflowExecution    │
//! │  - execute_node(node_id, input, cancel) → TaskExecution     │
//! │  - owns workflow and wasm engine                            │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   WorkflowExecution                         │
//! │  - wait() → WorkflowResult                                  │
//! │  - graph traversal, scheduling                              │
//! │  - input resolution via minijinja                           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use fuschia_engine::{WorkflowRuntime, WorkflowRunner, RuntimeConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! // Create the runtime
//! let config = RuntimeConfig {
//!     component_base_path: PathBuf::from("/path/to/components"),
//! };
//! let runtime = Arc::new(WorkflowRuntime::new(config, workflow)?);
//!
//! // Create a runner for the workflow
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

// Re-export from fuschia-workflow-runtime
pub use fuschia_workflow_runtime::{
  RuntimeConfig, RuntimeError, TaskExecution, TaskResult, WorkflowExecution, WorkflowResult,
  WorkflowRuntime,
};

pub use runner::WorkflowRunner;
