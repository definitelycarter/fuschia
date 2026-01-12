//! Trigger component execution.

use std::path::Path;

use fuscia_host::HostError;
use wasmtime::component::Component;
use wasmtime::{Engine, Store};

use crate::bindings::{Event, Status, TriggerComponent, TriggerHostState};

/// Result of executing a trigger component.
pub struct TriggerResult {
  /// The status returned by the trigger.
  pub status: Status,
}

/// Execute a trigger component.
///
/// # Arguments
/// * `engine` - Shared wasmtime Engine
/// * `component` - Pre-compiled wasmtime Component
/// * `state` - Host state with KV store and config
/// * `event` - The trigger event (poll or webhook)
/// * `epoch_deadline` - Optional epoch deadline for timeout
pub async fn execute_trigger(
  engine: &Engine,
  component: &Component,
  state: TriggerHostState,
  event: Event,
  epoch_deadline: Option<u64>,
) -> Result<TriggerResult, HostError> {
  let mut store = Store::new(engine, state);

  if let Some(deadline) = epoch_deadline {
    store.set_epoch_deadline(deadline);
  }

  // Create linker and add host imports
  let mut linker = wasmtime::component::Linker::<TriggerHostState>::new(engine);
  TriggerComponent::add_to_linker::<TriggerHostState, TriggerHostState>(&mut linker, |state| {
    state
  })?;

  // Instantiate the component
  let instance = TriggerComponent::instantiate_async(&mut store, component, &linker)
    .await
    .map_err(|e| HostError::instantiation(e.to_string()))?;

  // Call the trigger's handle function
  let result = instance
    .fuscia_trigger_trigger()
    .call_handle(&mut store, &event)
    .await?;

  match result {
    Ok(status) => Ok(TriggerResult { status }),
    Err(error_msg) => Err(HostError::component_error(error_msg)),
  }
}

/// Load and compile a component from a wasm file.
///
/// Components should be compiled once and reused for multiple executions.
pub fn load_component(engine: &Engine, wasm_path: &Path) -> Result<Component, HostError> {
  Component::from_file(engine, wasm_path).map_err(|e| HostError::instantiation(e.to_string()))
}
