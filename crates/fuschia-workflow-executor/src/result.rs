//! Workflow execution results.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// Re-export TaskResult from task-executor
pub use fuschia_task_executor::TaskResult;

/// Result of a complete workflow execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
  /// Unique execution ID.
  pub execution_id: String,
  /// Results of all executed tasks, keyed by node_id.
  pub task_results: HashMap<String, TaskResult>,
}
