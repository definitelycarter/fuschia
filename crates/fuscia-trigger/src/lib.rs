//! Trigger types and definitions for fuscia workflows.
//!
//! Triggers define how workflows are initiated. They can be:
//! - Manual: User-initiated execution
//! - Component: WebAssembly component that handles poll or webhook events
//!
//! The trigger type (poll/webhook) and configuration (interval, HTTP method)
//! are defined in the component manifest's TriggerExport.

mod types;

pub use types::{ComponentTrigger, Trigger, TriggerError, TriggerEvent};
