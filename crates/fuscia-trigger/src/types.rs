use fuscia_component::InstalledComponent;
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
  Manual(ManualTrigger),

  /// Webhook trigger - workflow is started by HTTP request
  Webhook(WebhookTrigger),

  /// Component trigger - wasm component that can register webhooks or poll
  Component(ComponentTrigger),
}

/// Manual trigger configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualTrigger {
  /// Optional schema for manual trigger inputs
  #[serde(default)]
  pub schema: Option<serde_json::Value>,
}

/// Webhook trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTrigger {
  /// HTTP method to accept (GET, POST, etc.)
  pub method: String,

  /// Path for the webhook endpoint
  pub path: String,

  /// Optional schema for validating webhook payload
  #[serde(default)]
  pub schema: Option<serde_json::Value>,
}

/// Component-based trigger configuration.
#[derive(Debug, Clone)]
pub struct ComponentTrigger {
  /// The installed component that implements the trigger
  pub component: InstalledComponent,

  /// Name of the trigger export within the component
  pub trigger_name: String,

  /// Configuration passed to the trigger component
  pub config: serde_json::Value,
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
    let trigger = Trigger::Manual(ManualTrigger { schema: None });
    assert!(matches!(trigger, Trigger::Manual(_)));
  }

  #[test]
  fn test_webhook_trigger() {
    let trigger = Trigger::Webhook(WebhookTrigger {
      method: "POST".to_string(),
      path: "/webhooks/my-workflow".to_string(),
      schema: None,
    });
    assert!(matches!(trigger, Trigger::Webhook(_)));
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
