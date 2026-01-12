//! Trigger types and definitions for fuscia workflows.
//!
//! Triggers define how workflows are initiated. They can be:
//! - Manual: User-initiated execution
//! - Webhook: HTTP endpoint that starts the workflow
//! - Component: WebAssembly component that can register webhooks or poll external services

mod types;

pub use types::{
  ComponentTrigger, ManualTrigger, Trigger, TriggerError, TriggerEvent, WebhookTrigger,
};
