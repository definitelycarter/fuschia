use crate::error::TaskError;
use crate::types::{Task, TaskContext, TaskOutput};
use crate::{component, http};

/// Execute a task.
///
/// Routes to the appropriate executor based on task type.
pub async fn execute(task: &Task, ctx: &TaskContext) -> Result<TaskOutput, TaskError> {
  match task {
    Task::Http => http::execute(ctx).await,
    Task::Component { component } => component::execute(component, ctx).await,
  }
}
