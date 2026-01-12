use std::collections::HashMap;

use fuscia_config::{ExecutionMode, InputValue, JoinStrategy, LoopFailureMode, TriggerType};
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

/// A locked trigger that initiates workflow execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedTrigger {
  /// The type of trigger (manual, poll, webhook)
  pub trigger_type: TriggerType,

  /// Optional component for custom trigger processing.
  /// None for basic built-in triggers (manual, simple webhook/poll).
  /// Some for component-authored triggers with validation/transformation logic.
  pub component: Option<LockedTriggerComponent>,
}

/// A locked trigger component with its export name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedTriggerComponent {
  /// The locked component that implements the trigger
  pub component: LockedComponent,
  /// Name of the trigger export within the component (e.g., "row-added")
  pub trigger_name: String,
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
