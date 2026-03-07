# Node Data

JSON is the primary serialization format for data flowing between nodes.

## Node Data Envelope

Each node receives and produces data in a structured envelope:

```json
{
  "workflow_id": "wf_abc",
  "node_id": "send_email",
  "task_id": "task_789",
  "started_at": "2026-01-10T12:00:00Z",
  "artifacts": [...],
  "output": { ... }
}
```

| Field | Description |
|-------|-------------|
| `workflow_id` | Unique identifier for the workflow definition |
| `node_id` | Identifier for the node in the graph (static) |
| `task_id` | Unique identifier for this execution of the node (dynamic) |
| `started_at` | ISO 8601 timestamp |
| `artifacts` | Array of artifact references produced |
| `output` | Dynamic JSON output from the node |

### Why `node_id` and `task_id` are Separate

A `node_id` identifies a step in the workflow definition. A `task_id` identifies a specific execution. This matters for loops: a single `node_id` generates multiple `task_id`s (one per iteration).

## Artifact System

Binary and large data are handled through artifacts. All artifacts are stored as references — there are no inline artifacts.

### Artifact Reference

```json
{
  "artifact_id": "art_12345",
  "content_type": "image/png"
}
```

### Storage Model

1. Node produces an artifact → immediately stored (filesystem locally, S3 in production)
2. Node's output contains a reference (`artifact_id`, `content_type`)
3. Downstream nodes access the artifact by ID through the host interface

Artifacts are **not passed through** envelopes. They are stored in the execution context and available by reference. This keeps envelopes lightweight.

### Host Interface

Components interact with artifacts via host imports:
- **Read artifact**: fetch content by ID
- **Write artifact**: store content, receive an artifact ID

Components never see raw filesystem paths — they work with artifact IDs that the engine resolves internally.

### Cleanup

Artifacts are not automatically deleted. Manual cleanup via CLI:
- Query completed workflows by time range
- Delete artifacts for specific workflows
- Execution history is preserved separately from artifacts

## Node Output Model

Nodes use a request/response model: receive input, execute, return output.

The output type is designed for forward-compatible streaming:

```wit
variant node-output {
    complete(output-data),
    // Future: stream-start(stream-handle),
    // Future: stream-chunk(stream-handle, output-data),
    // Future: stream-end(stream-handle),
}
```

Existing nodes return `complete(...)`. When streaming is added, the variant is extended without breaking existing components.
