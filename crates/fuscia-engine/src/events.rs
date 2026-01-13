//! Execution events and notifiers for observability.
//!
//! Events are emitted during workflow execution to allow consumers to observe
//! progress, persist state, stream to UIs, etc.

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Events emitted during workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEvent {
  /// Workflow execution has started.
  WorkflowStarted {
    execution_id: String,
    workflow_id: String,
  },

  /// A node has started executing.
  NodeStarted {
    execution_id: String,
    node_id: String,
  },

  /// A node has completed successfully.
  NodeCompleted {
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
  },

  /// A node has failed.
  NodeFailed {
    execution_id: String,
    node_id: String,
    error: String,
  },

  /// Workflow execution has completed successfully.
  WorkflowCompleted { execution_id: String },

  /// Workflow execution has failed.
  WorkflowFailed { execution_id: String, error: String },
}

/// Trait for receiving execution events.
///
/// Implement this trait to receive events during workflow execution.
/// The engine calls `notify` for each event - implementations decide
/// what to do with them (persist, broadcast, log, ignore, etc.).
pub trait ExecutionNotifier: Send + Sync {
  /// Called when an execution event occurs.
  fn notify(&self, event: ExecutionEvent);
}

/// A no-op notifier that discards all events.
///
/// Useful for tests or when event observation is not needed.
#[derive(Debug, Clone, Default)]
pub struct NoopNotifier;

impl ExecutionNotifier for NoopNotifier {
  fn notify(&self, _event: ExecutionEvent) {
    // Intentionally empty
  }
}

/// A notifier that sends events to an unbounded channel.
///
/// Use this when you need to consume events asynchronously (e.g., persist
/// to a database, stream to a UI via websocket, etc.).
#[derive(Debug, Clone)]
pub struct ChannelNotifier {
  // NOTE: We use an unbounded channel here to avoid blocking the engine if the
  // consumer is slow. The event volume is low (one per node start/complete), so
  // memory growth is unlikely in practice. If this becomes a concern, alternatives:
  // - Use bounded channel and accept backpressure (engine waits for slow consumer)
  // - Use try_send and drop events if buffer is full
  // - Buffer events internally and flush periodically
  sender: mpsc::UnboundedSender<ExecutionEvent>,
}

impl ChannelNotifier {
  /// Create a new channel notifier.
  pub fn new(sender: mpsc::UnboundedSender<ExecutionEvent>) -> Self {
    Self { sender }
  }
}

impl ExecutionNotifier for ChannelNotifier {
  fn notify(&self, event: ExecutionEvent) {
    // Ignore send errors - receiver may have been dropped
    let _ = self.sender.send(event);
  }
}
