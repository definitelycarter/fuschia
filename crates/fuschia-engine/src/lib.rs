//! Fuschia Workflow Engine
//!
//! This crate provides the workflow execution engine for fuschia.
//! It handles graph traversal, input resolution, and component execution.
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
//! │                      WorkflowEngine                         │
//! │  - execute(workflow, payload) → ExecutionResult             │
//! │  - graph traversal, scheduling                              │
//! │  - input resolution via minijinja                           │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    fuschia-task-host                         │
//! │  - execute_task(engine, component, state, ctx, data)        │
//! │  - handles wasm instantiation + execution                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use fuschia_engine::{WorkflowEngine, WorkflowRunner, EngineConfig};
//! use tokio_util::sync::CancellationToken;
//!
//! // Create the engine
//! let config = EngineConfig {
//!     component_base_path: PathBuf::from("/path/to/components"),
//! };
//! let engine = Arc::new(WorkflowEngine::new(config)?);
//!
//! // Create a runner for a workflow
//! let runner = WorkflowRunner::new(workflow, engine);
//!
//! // Get sender for external triggers
//! let sender = runner.sender();
//!
//! // Start the execution loop
//! let cancel = CancellationToken::new();
//! runner.start(cancel).await?;
//! ```
//!
//! # Input Resolution
//!
//! Node inputs are resolved using minijinja templates:
//!
//! **Single upstream node:**
//! ```json
//! { "recipient": "{{ email }}", "greeting": "Hello {{ name | title }}!" }
//! ```
//!
//! **Join node (multiple upstream):**
//! ```json
//! { "user_email": "{{ fetch_user.email }}", "config": "{{ get_config.setting }}" }
//! ```

mod cache;
mod engine;
mod error;
mod events;
mod input;
mod runner;

pub use cache::{ComponentCache, ComponentKey};
pub use engine::{EngineConfig, ExecutionResult, NodeResult, WorkflowEngine};
pub use error::ExecutionError;
pub use events::{ChannelNotifier, ExecutionEvent, ExecutionNotifier, NoopNotifier};
pub use input::{SchemaType, coerce_inputs, resolve_inputs};
pub use runner::WorkflowRunner;
