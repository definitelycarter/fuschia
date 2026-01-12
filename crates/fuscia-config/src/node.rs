use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::component::ComponentRef;
use crate::edge::Edge;
use crate::enums::{ExecutionMode, JoinStrategy, LoopFailureMode, TriggerType};
use crate::input::InputValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDef {
  pub node_id: String,
  #[serde(flatten)]
  pub node_type: NodeType,
  #[serde(default)]
  pub inputs: HashMap<String, InputValue>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeout_ms: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_retry_attempts: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub fail_workflow: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeType {
  /// A trigger node that initiates workflow execution.
  Trigger {
    /// The type of trigger (manual, poll, webhook)
    #[serde(flatten)]
    trigger_type: TriggerType,
    /// Optional component for custom trigger processing
    #[serde(skip_serializing_if = "Option::is_none")]
    component: Option<ComponentRef>,
    /// Name of the trigger export within the component
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger_name: Option<String>,
  },
  #[serde(rename = "component")]
  Component {
    #[serde(flatten)]
    component: ComponentRef,
  },
  Join {
    join_strategy: JoinStrategy,
  },
  Loop {
    execution_mode: ExecutionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrency: Option<u32>,
    failure_mode: LoopFailureMode,
    nodes: Vec<NodeDef>,
    edges: Vec<Edge>,
  },
}
