//! Fuscia Store
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

/// Storage trait for workflow executions and tasks.
pub trait Store {
  /// Error type for storage operations.
  type Error;

  /// Create a new workflow execution.
  fn create_execution(
    &self,
    execution: &WorkflowExecution,
  ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

  /// Get a workflow execution by ID.
  fn get_execution(
    &self,
    execution_id: &str,
  ) -> impl std::future::Future<Output = Result<WorkflowExecution, Self::Error>> + Send;

  /// Update the status of a workflow execution.
  fn update_execution_status(
    &self,
    execution_id: &str,
    status: ExecutionStatus,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

  /// List executions for a workflow.
  fn list_executions(
    &self,
    workflow_id: &str,
  ) -> impl std::future::Future<Output = Result<Vec<WorkflowExecution>, Self::Error>> + Send;

  /// Create a new task.
  fn create_task(
    &self,
    task: &Task,
  ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

  /// Get a task by ID.
  fn get_task(
    &self,
    task_id: &str,
  ) -> impl std::future::Future<Output = Result<Task, Self::Error>> + Send;

  /// Update a task.
  fn update_task(
    &self,
    task: &Task,
  ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

  /// List tasks for an execution.
  fn list_tasks(
    &self,
    execution_id: &str,
  ) -> impl std::future::Future<Output = Result<Vec<Task>, Self::Error>> + Send;
}
