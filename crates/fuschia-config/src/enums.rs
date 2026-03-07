use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinStrategy {
  All,
  Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
  Sequential,
  Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopFailureMode {
  StopAndFail,
  StopWithErrors,
  Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryBackoff {
  Constant,
  Linear,
  Exponential,
}

/// The runtime backend a component targets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    #[default]
    Wasm,
    Lua,
    Js,
}

/// The type of trigger and its configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "trigger_type", rename_all = "snake_case")]
pub enum TriggerType {
  /// Manual trigger - user-initiated via CLI or UI
  Manual,
  /// Poll-based trigger that fires on a schedule
  Poll {
    /// Polling interval in milliseconds
    interval_ms: u64,
  },
  /// Webhook-based trigger that fires on HTTP request
  Webhook {
    /// HTTP method to accept (GET, POST, etc.)
    method: String,
  },
}
