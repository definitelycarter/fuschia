# Error Handling

## Component Execution Errors

```rust
enum HostError {
    /// Component instantiation failed (bad wasm, missing imports)
    Instantiation { message: String },
    /// Component trapped (OOM, stack overflow, epoch timeout)
    Trap { trap_code: Option<TrapCode>, message: String },
    /// Component returned error via WIT result type
    ComponentError { message: String },
    /// Invalid resource table handle
    Resource { message: String },
}
```

`ComponentError` doesn't automatically mean task failure. A component can return an error without trapping, and the engine decides based on workflow config whether it's "failed" or "completed with errors".

## Runtime Errors

```rust
enum RuntimeError {
    Cancelled,
    InputSerialization { message: String },
    ComponentExecution { source: HostError },
    InvalidOutput { message: String },
    InputResolution { node_id: String, message: String },
    ComponentLoad { node_id: String, message: String },
    InvalidGraph { message: String },
}
```

## Retry Policies

Configurable at two levels:

| Level | Description |
|-------|-------------|
| Workflow | Default retry policy for all nodes |
| Node | Override policy for a specific node (completely replaces workflow default) |

Backoff strategies: `constant`, `linear`, `exponential`.

## Failure Behavior

By default, a node failure (after retries exhausted) fails the entire workflow.

Nodes can be marked as non-critical with `fail_workflow: false`:
- Failure is logged
- Node is marked as failed in execution record
- Other branches continue

## Workflow Execution Status

| Status | Description |
|--------|-------------|
| `succeeded` | All nodes completed successfully |
| `failed` | A critical node failed (after retries exhausted) |
| `completed_with_errors` | Workflow finished but one or more non-critical nodes failed |
