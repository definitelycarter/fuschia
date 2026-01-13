//! Fuschia Store
//!
//! This crate provides the storage trait and implementations for workflow
//! executions and tasks. Data is persisted to a database (SQLite or PostgreSQL).
//!
//! The [`Store`] trait defines operations for:
//! - Creating and updating workflow executions
//! - Creating and updating task records
//! - Querying execution history

mod sqlite;
mod types;

pub use sqlite::SqliteStore;
pub use types::{ExecutionStatus, Task, TaskStatus, WorkflowExecution};

use async_trait::async_trait;

/// Error type for storage operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// The requested record was not found.
  #[error("not found: {0}")]
  NotFound(String),

  /// A database error occurred.
  #[error("database error: {0}")]
  Database(#[from] sqlx::Error),
}

/// Storage trait for workflow executions and tasks.
#[async_trait]
pub trait Store: Send + Sync {
  /// Create a new workflow execution.
  async fn create_execution(&self, execution: &WorkflowExecution) -> Result<(), Error>;

  /// Get a workflow execution by ID.
  async fn get_execution(&self, execution_id: &str) -> Result<WorkflowExecution, Error>;

  /// Update the status of a workflow execution.
  async fn update_execution_status(
    &self,
    execution_id: &str,
    status: ExecutionStatus,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<(), Error>;

  /// List executions for a workflow.
  async fn list_executions(&self, workflow_id: &str) -> Result<Vec<WorkflowExecution>, Error>;

  /// Create a new task.
  async fn create_task(&self, task: &Task) -> Result<(), Error>;

  /// Get a task by ID.
  async fn get_task(&self, task_id: &str) -> Result<Task, Error>;

  /// Update a task.
  async fn update_task(&self, task: &Task) -> Result<(), Error>;

  /// List tasks for an execution.
  async fn list_tasks(&self, execution_id: &str) -> Result<Vec<Task>, Error>;
}
