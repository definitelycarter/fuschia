use fuschia_component_registry::InstalledComponent;
use serde::{Deserialize, Serialize};

/// Error type for trigger operations.
#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
  #[error("trigger not found: {0}")]
  NotFound(String),

  #[error("invalid trigger configuration: {0}")]
  InvalidConfig(String),

  #[error("trigger component error: {0}")]
  ComponentError(String),
}

/// A trigger that can initiate workflow execution.
#[derive(Debug, Clone)]
pub enum Trigger {
  /// Manual trigger - workflow is started by user action
  Manual,

  /// Component trigger - wasm component that handles poll or webhook events
  Component(Box<ComponentTrigger>),
}

/// Component trigger - wasm component that handles events.
///
/// The trigger type (poll/webhook) and its configuration (interval, method)
/// are defined in the component manifest's TriggerExport.
#[derive(Debug, Clone)]
pub struct ComponentTrigger {
  /// The installed component that implements the trigger
  pub component: InstalledComponent,

  /// Name of the trigger export within the component
  pub trigger_name: String,

  /// Input values for the trigger (conforms to the trigger's schema)
  pub inputs: serde_json::Value,
}

/// Event emitted by a trigger to start workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
  /// Unique identifier for this trigger event
  pub event_id: String,

  /// The trigger that emitted this event
  pub trigger_id: String,

  /// Payload data from the trigger
  pub payload: serde_json::Value,

  /// Timestamp when the event was emitted (Unix millis)
  pub timestamp: u64,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_manual_trigger() {
    let trigger = Trigger::Manual;
    assert!(matches!(trigger, Trigger::Manual));
  }

  #[test]
  fn test_trigger_event() {
    let event = TriggerEvent {
      event_id: "evt_123".to_string(),
      trigger_id: "trg_456".to_string(),
      payload: serde_json::json!({"key": "value"}),
      timestamp: 1704067200000,
    };

    assert_eq!(event.event_id, "evt_123");
    assert_eq!(event.payload["key"], "value");
  }
}
