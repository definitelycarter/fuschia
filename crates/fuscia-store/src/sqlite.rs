use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::{ExecutionStatus, Store, Task, WorkflowExecution};

/// SQLite-based store implementation.
pub struct SqliteStore {
  pool: SqlitePool,
}

impl SqliteStore {
  /// Create a new SQLite store with the given connection pool.
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  /// Run database migrations.
  pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(&self.pool).await
  }
}

impl Store for SqliteStore {
  type Error = sqlx::Error;

  async fn create_execution(&self, execution: &WorkflowExecution) -> Result<(), Self::Error> {
    sqlx::query(
            r#"
            INSERT INTO workflow_executions (execution_id, workflow_id, status, config, started_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&execution.execution_id)
        .bind(&execution.workflow_id)
        .bind(&execution.status)
        .bind(&execution.config)
        .bind(&execution.started_at)
        .bind(&execution.completed_at)
        .execute(&self.pool)
        .await?;

    Ok(())
  }

  async fn get_execution(&self, execution_id: &str) -> Result<WorkflowExecution, Self::Error> {
    sqlx::query_as(
      r#"
            SELECT execution_id, workflow_id, status, config, started_at, completed_at
            FROM workflow_executions
            WHERE execution_id = ?
            "#,
    )
    .bind(execution_id)
    .fetch_one(&self.pool)
    .await
  }

  async fn update_execution_status(
    &self,
    execution_id: &str,
    status: ExecutionStatus,
    completed_at: Option<DateTime<Utc>>,
  ) -> Result<(), Self::Error> {
    sqlx::query(
      r#"
            UPDATE workflow_executions
            SET status = ?, completed_at = ?
            WHERE execution_id = ?
            "#,
    )
    .bind(status)
    .bind(completed_at)
    .bind(execution_id)
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn list_executions(
    &self,
    workflow_id: &str,
  ) -> Result<Vec<WorkflowExecution>, Self::Error> {
    sqlx::query_as(
      r#"
            SELECT execution_id, workflow_id, status, config, started_at, completed_at
            FROM workflow_executions
            WHERE workflow_id = ?
            ORDER BY started_at DESC
            "#,
    )
    .bind(workflow_id)
    .fetch_all(&self.pool)
    .await
  }

  async fn create_task(&self, task: &Task) -> Result<(), Self::Error> {
    sqlx::query(
            r#"
            INSERT INTO workflow_tasks (task_id, execution_id, node_id, status, attempt, started_at, completed_at, output, error)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.task_id)
        .bind(&task.execution_id)
        .bind(&task.node_id)
        .bind(&task.status)
        .bind(task.attempt)
        .bind(&task.started_at)
        .bind(&task.completed_at)
        .bind(&task.output)
        .bind(&task.error)
        .execute(&self.pool)
        .await?;

    Ok(())
  }

  async fn get_task(&self, task_id: &str) -> Result<Task, Self::Error> {
    sqlx::query_as(
            r#"
            SELECT task_id, execution_id, node_id, status, attempt, started_at, completed_at, output, error
            FROM workflow_tasks
            WHERE task_id = ?
            "#,
        )
        .bind(task_id)
        .fetch_one(&self.pool)
        .await
  }

  async fn update_task(&self, task: &Task) -> Result<(), Self::Error> {
    sqlx::query(
      r#"
            UPDATE workflow_tasks
            SET status = ?, attempt = ?, completed_at = ?, output = ?, error = ?
            WHERE task_id = ?
            "#,
    )
    .bind(&task.status)
    .bind(task.attempt)
    .bind(&task.completed_at)
    .bind(&task.output)
    .bind(&task.error)
    .bind(&task.task_id)
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn list_tasks(&self, execution_id: &str) -> Result<Vec<Task>, Self::Error> {
    sqlx::query_as(
            r#"
            SELECT task_id, execution_id, node_id, status, attempt, started_at, completed_at, output, error
            FROM workflow_tasks
            WHERE execution_id = ?
            ORDER BY started_at ASC
            "#,
        )
        .bind(execution_id)
        .fetch_all(&self.pool)
        .await
  }
}
