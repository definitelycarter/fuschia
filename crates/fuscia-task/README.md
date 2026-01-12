# fuscia-task

Task execution for fuscia workflows. Handles running individual tasks with resolved inputs.

## Key Types

| Type | Description |
|------|-------------|
| `Task` | Enum of task types (Http, Component) |
| `TaskContext` | Execution context with identity and resolved inputs |
| `TaskOutput` | Result containing output JSON and artifact references |
| `TaskError` | Error types for task execution failures |

## Task Types

### HTTP

Makes HTTP requests. Inputs:

```json
{
  "method": "POST",
  "url": "https://api.example.com/data",
  "headers": { "Authorization": "Bearer ..." },
  "body": { "key": "value" }
}
```

Output:

```json
{
  "status": 200,
  "headers": { "content-type": "application/json" },
  "body": { "result": "..." }
}
```

### Component

Executes a Wasm component. Requires `fuscia-host` (not yet implemented).

```rust
Task::Component {
    component: InstalledComponent { ... }
}
```

The component's capabilities (allowed hosts, etc.) are enforced by the host.

## Usage

```rust
use fuscia_task::{execute, Task, TaskContext};

let task = Task::Http;
let ctx = TaskContext {
    execution_id: "exec-123".to_string(),
    node_id: "fetch-data".to_string(),
    task_id: "task-456".to_string(),
    inputs: serde_json::json!({
        "method": "GET",
        "url": "https://api.example.com/users"
    }),
};

let output = execute(&task, &ctx).await?;
println!("Status: {}", output.output["status"]);
```

## Design Notes

- **Inputs are pre-resolved**: Path expressions like `$.node.output.field` are resolved by the engine before reaching this crate. Tasks receive concrete JSON values.
- **Stateless execution**: Each task execution is independent. No state is shared between executions.
- **Artifacts**: Tasks can produce artifacts (binary data stored separately). These are returned as `ArtifactRef` in the output.
