use crate::error::TaskError;
use crate::types::{TaskContext, TaskOutput};
use fuscia_component_registry::InstalledComponent;

/// Execute a Wasm component task.
///
/// This is a stub that will be replaced when `fuscia-host` is implemented.
/// The actual implementation will:
/// 1. Load the wasm component from `component.wasm_path`
/// 2. Set up wasmtime with capability restrictions from `component.manifest.capabilities`
/// 3. Execute the component with `ctx.inputs`
/// 4. Return the component's output
pub async fn execute(
  _component: &InstalledComponent,
  _ctx: &TaskContext,
) -> Result<TaskOutput, TaskError> {
  // TODO: Implement via fuscia-host
  Err(TaskError::Component {
    message: "component execution not yet implemented - needs fuscia-host".to_string(),
  })
}
