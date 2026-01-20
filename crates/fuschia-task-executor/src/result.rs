//! Task execution result.

use serde::{Deserialize, Serialize};

/// Result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
  /// Unique task ID.
  pub task_id: String,
  /// Node ID this task executed.
  pub node_id: String,
  /// Input from upstream node(s).
  pub input: serde_json::Value,
  /// Input after template resolution.
  pub resolved_input: serde_json::Value,
  /// Task output.
  pub output: serde_json::Value,
}
