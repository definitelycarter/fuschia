use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::Json;

/// Status of a workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ExecutionStatus {
  Running,
  Succeeded,
  Failed,
  CompletedWithErrors,
}

/// Status of a task (node execution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum TaskStatus {
  Pending,
  Running,
  Succeeded,
  Failed,
}

/// A workflow execution as stored in the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct WorkflowExecution {
  pub execution_id: String,
  pub workflow_id: String,
  pub status: ExecutionStatus,
  pub config: Json<serde_json::Value>,
  pub started_at: DateTime<Utc>,
  pub completed_at: Option<DateTime<Utc>>,
}

/// A task (node execution) as stored in the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct Task {
  pub task_id: String,
  pub execution_id: String,
  pub node_id: String,
  pub status: TaskStatus,
  pub attempt: i32,
  pub started_at: DateTime<Utc>,
  pub completed_at: Option<DateTime<Utc>>,
  pub output: Option<Json<serde_json::Value>>,
  pub error: Option<String>,
}
