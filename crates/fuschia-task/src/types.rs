use fuschia_component_registry::InstalledComponent;
use serde::{Deserialize, Serialize};

/// A task to be executed, derived from a workflow node.
#[derive(Debug, Clone)]
pub enum Task {
  /// HTTP request task. Configuration comes from inputs:
  /// - method: "GET", "POST", etc.
  /// - url: target URL
  /// - headers: optional headers map
  /// - body: optional request body
  Http,

  /// Wasm component task.
  Component {
    /// The installed component to execute.
    component: Box<InstalledComponent>,
  },
}

/// Context provided to a task during execution.
#[derive(Debug, Clone)]
pub struct TaskContext {
  /// Workflow execution ID.
  pub execution_id: String,

  /// Node ID within the workflow.
  pub node_id: String,

  /// Unique task ID for this execution.
  pub task_id: String,

  /// Resolved inputs for the task.
  pub inputs: serde_json::Value,
}

/// Output produced by a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
  /// The task's output data.
  pub output: serde_json::Value,

  /// Artifacts produced by the task.
  pub artifacts: Vec<ArtifactRef>,
}

/// Reference to an artifact produced by a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
  /// Unique artifact ID.
  pub artifact_id: String,

  /// MIME type of the artifact.
  pub content_type: String,
}
