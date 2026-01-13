//! Task component execution.

use std::path::Path;

use fuschia_host::HostError;
use wasmtime::component::Component;
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::add_to_linker_async;

use crate::bindings::{Context, Output, TaskComponent, TaskHostState};

/// Result of executing a task component.
pub struct TaskResult {
  /// The output data from the component (JSON string).
  pub output: Output,
}

/// Execute a task component.
///
/// # Arguments
/// * `engine` - Shared wasmtime Engine
/// * `component` - Pre-compiled wasmtime Component
/// * `state` - Host state with KV store and config
/// * `ctx` - Task execution context (execution_id, node_id, task_id)
/// * `data` - Input data as JSON string
/// * `epoch_deadline` - Optional epoch deadline for timeout
pub async fn execute_task(
  engine: &Engine,
  component: &Component,
  state: TaskHostState,
  ctx: Context,
  data: String,
  epoch_deadline: Option<u64>,
) -> Result<TaskResult, HostError> {
  let mut store = Store::new(engine, state);

  // Set epoch deadline - use provided value or a high default to avoid immediate interrupt
  // when epoch interruption is enabled on the engine
  let deadline = epoch_deadline.unwrap_or(u64::MAX);
  store.set_epoch_deadline(deadline);

  // Create linker and add host imports
  let mut linker = wasmtime::component::Linker::<TaskHostState>::new(engine);

  // Add WASI imports
  add_to_linker_async(&mut linker)?;

  // Add fuschia host imports
  TaskComponent::add_to_linker::<TaskHostState, TaskHostState>(&mut linker, |state| state)?;

  // Instantiate the component
  let instance = TaskComponent::instantiate_async(&mut store, component, &linker)
    .await
    .map_err(|e| HostError::instantiation(e.to_string()))?;

  // Call the task's execute function
  let result = instance
    .fuschia_task_task()
    .call_execute(&mut store, &ctx, &data)
    .await?;

  match result {
    Ok(output) => Ok(TaskResult { output }),
    Err(error_msg) => Err(HostError::component_error(error_msg)),
  }
}

/// Load and compile a component from a wasm file.
///
/// Components should be compiled once and reused for multiple executions.
pub fn load_component(engine: &Engine, wasm_path: &Path) -> Result<Component, HostError> {
  Component::from_file(engine, wasm_path).map_err(|e| HostError::instantiation(e.to_string()))
}
