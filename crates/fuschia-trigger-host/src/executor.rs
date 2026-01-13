//! Trigger component execution.

use std::path::Path;

use fuschia_host::HostError;
use wasmtime::component::Component;
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::add_to_linker_async;

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

  // Set epoch deadline - use provided value or a high default to avoid immediate interrupt
  // when epoch interruption is enabled on the engine
  let deadline = epoch_deadline.unwrap_or(u64::MAX);
  store.set_epoch_deadline(deadline);

  // Create linker and add host imports
  let mut linker = wasmtime::component::Linker::<TriggerHostState>::new(engine);

  // Add WASI imports
  add_to_linker_async(&mut linker)?;

  // Add fuschia host imports
  TriggerComponent::add_to_linker::<TriggerHostState, TriggerHostState>(&mut linker, |state| {
    state
  })?;

  // Instantiate the component
  let instance = TriggerComponent::instantiate_async(&mut store, component, &linker)
    .await
    .map_err(|e| HostError::instantiation(e.to_string()))?;

  // Call the trigger's handle function
  let result = instance
    .fuschia_trigger_trigger()
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
