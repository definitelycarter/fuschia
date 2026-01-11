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
  Component(LockedComponent),
  Join { strategy: JoinStrategy },
  Loop(LockedLoop),
}

/// A reference to content (e.g., a Wasm component).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentRef {
  /// URI where the content can be fetched (file://, https://, oci://, etc.)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<String>,

  /// SHA-256 digest of the content (sha256:abc123...)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub digest: Option<String>,
}

/// A component that has been resolved and locked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedComponent {
  /// Reference to the Wasm component bytes.
  pub content: ContentRef,
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
