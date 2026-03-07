# Tasks

Tasks are the units of work within a workflow. Each task node executes code in a sandboxed runtime and produces JSON output.

## Task Types

| Type | Description |
|------|-------------|
| Component | Executes a WebAssembly component, Lua script, or JS script |
| Http | Built-in HTTP request executor (no component needed) |

## WIT Interface

Component tasks implement the `execute` function:

```wit
interface task {
    record context {
        execution-id: string,
        node-id: string,
        task-id: string,
    }

    record output {
        data: string,  // JSON
    }

    execute: func(ctx: context, data: string) -> result<output, string>;
}
```

## Task Context

When a task executes, it receives:

- **execution-id**: Unique identifier for this workflow execution
- **node-id**: Identifier for the node in the workflow graph (static across executions)
- **task-id**: Unique identifier for this specific task execution (unique per run, important for loops)

## Input

Inputs are passed as a pre-resolved JSON string. The engine handles all template resolution and type coercion before the task sees the data. The task receives clean, typed JSON.

## Output

Tasks return an `output` with a `data` field containing JSON. This output becomes available to downstream nodes via template expressions.

## Http Task

The built-in HTTP executor parses inputs for:
- `method` — HTTP method (GET, POST, etc.)
- `url` — Request URL
- `headers` — Optional headers map
- `body` — Optional request body

Returns status code, response headers, and body as JSON output.

## Host Imports

Tasks have access to host-provided capabilities:

| Import | Description |
|--------|-------------|
| KV store | Persist state across invocations within an execution |
| Config | Read-only configuration from workflow node definition |
| Log | Structured logging routed to OpenTelemetry |
| HTTP | Outbound HTTP requests (filtered by allowed_hosts) |
