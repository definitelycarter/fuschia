use std::collections::HashMap;

use fuscia_config::{ExecutionMode, InputValue, JoinStrategy, LoopFailureMode};
use serde::{Deserialize, Serialize};

/// A resolved node in a locked workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
  pub node_id: String,
  pub node_type: NodeType,
  pub inputs: HashMap<String, InputValue>,
  pub timeout_ms: Option<u64>,
  pub max_retry_attempts: Option<u32>,
  pub fail_workflow: bool,
}

/// The type of a resolved node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
  /// A trigger node that initiates workflow execution.
  /// Trigger nodes cannot have incoming edges.
  Trigger(LockedTrigger),
  /// A component task node.
  Component(LockedComponent),
  /// A join node that merges multiple branches.
  Join { strategy: JoinStrategy },
  /// A loop node with a nested workflow.
  Loop(LockedLoop),
}

/// A trigger component that has been resolved and locked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedTrigger {
  /// Component name (e.g., "my-org/webhook-trigger")
  pub name: String,

  /// Component version (e.g., "1.0.0")
  pub version: String,

  /// SHA-256 digest of the wasm binary (for verification)
  pub digest: String,
}

/// A component that has been resolved and locked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedComponent {
  /// Component name (e.g., "my-org/sentiment-analysis")
  pub name: String,

  /// Component version (e.g., "1.0.0")
  pub version: String,

  /// SHA-256 digest of the wasm binary (for verification)
  pub digest: String,
}

/// A loop node with its nested workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedLoop {
  pub execution_mode: ExecutionMode,
  pub concurrency: Option<u32>,
  pub failure_mode: LoopFailureMode,
  /// The nested workflow inside the loop.
  pub workflow: Box<crate::Workflow>,
}
