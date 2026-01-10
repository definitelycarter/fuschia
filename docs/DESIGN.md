# Fuscia Design Document

Fuscia is a workflow engine similar to n8n, built on WebAssembly components using WIT (WebAssembly Interface Types). Each workflow node is a Wasm component with explicitly defined capabilities.

## Execution Model

### Graph Structure

Workflows are represented as **directed graphs with control flow**, not strict DAGs. This supports:

- **Loop nodes** - iterate over items or repeat until a condition is met
- **IF/Switch nodes** - conditional branching
- **Cycles** - a node's output can connect back to an earlier node

### Parallel Execution

Independent branches execute in parallel. When a workflow splits into multiple paths that don't depend on each other, the engine spawns concurrent tasks for each branch and synchronizes at join points where branches merge.

### Node Input Mappings

Each node specifies where its inputs come from via explicit mappings. Inputs reference prior node outputs using path expressions:

```json
{
  "node_id": "send_email",
  "component": "email-sender",
  "inputs": {
    "recipient": "$.fetch_user.output.email",
    "subject": "$.generate_content.output.title",
    "attachment": "$.process_file.artifacts[0]"
  }
}
```

The engine resolves these mappings at runtime by pulling from prior nodes' outputs and artifacts.

### Join Nodes

Join nodes are special nodes that synchronize parallel branches. Unlike regular nodes (which receive input from one upstream node), join nodes receive a map of all incoming branch outputs.

**Join Input Structure:**
```json
{
  "inputs": {
    "branch_a_node": { "status": "succeeded", "output": {...}, "artifacts": [...] },
    "branch_b_node": { "status": "succeeded", "output": {...}, "artifacts": [...] },
    "branch_c_node": { "status": "failed", "error": "..." }
  }
}
```

**Join Node Types:**

```rust
enum JoinNodeType {
    Strategy(JoinStrategy),
    Custom(WasmComponent),
}

enum JoinStrategy {
    All, // Proceed only if all branches succeeded
    Any, // Proceed if at least one branch succeeded
}
```

Both strategies wait for all branches to complete before evaluating. Built-in strategies control **whether to proceed**. The join node must still produce a single output envelope for downstream nodes, either via the strategy's default behavior or custom Wasm logic.

### Loop Nodes

Loop nodes are container nodes that iterate over a collection and execute nested nodes for each element.

**Structure:**
```
[Fetch Users] → [Loop Node] → [Send Summary]
                    │
                    └── nested: [Process User] → [Update DB]
```

**Input:** Array or Object
- Array: iterates over elements
- Object: iterates over key-value pairs (useful for Join node output)

**Item Context:**

Each iteration receives context about the current item:
```json
// Array iteration
{ "item": {...}, "index": 0 }

// Object iteration
{ "item": {...}, "key": "branch_a" }
```

**Execution Mode:** Configurable per-loop
- Sequential: one item at a time
- Parallel: concurrent execution with configurable concurrency limit

**Failure Modes:**

| Mode | Behavior |
|------|----------|
| `stop_and_fail` | Stop processing, fail the loop node |
| `stop_with_errors` | Stop processing, complete with errors status |
| `continue` | Log error, continue to next item |

**Output:** Array of iteration results

**Future Enhancement:** When streaming is added, loop nodes can receive/emit streams for large collections, enabling lazy iteration and backpressure.

## Data Format

### Serialization

JSON is the primary serialization format for data flowing between nodes. Binary data is handled via the artifact system (see below).

### Node Data Envelope

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
| `node_id` | Identifier for the node/step in the workflow graph (static) |
| `task_id` | Unique identifier for this execution of the node (dynamic) |
| `started_at` | ISO 8601 timestamp when this task started |
| `artifacts` | Array of artifacts produced or passed through |
| `output` | Dynamic JSON output from the node |

### Why `node_id` and `task_id` are Separate

A `node_id` identifies a step in the workflow definition. A `task_id` identifies a specific execution of that step. This distinction matters for loops: a single `node_id` may generate multiple `task_id`s (one per iteration).

## Artifact System

Binary and large data are handled through artifacts. All artifacts are stored as references - there are no inline artifacts.

### Artifact Structure

```json
{
  "artifact_id": "art_12345",
  "content_type": "image/png"
}
```

### Storage Model

When a node produces an artifact:
1. The artifact is immediately stored (filesystem locally, S3 in production)
2. The node's output contains a reference (`artifact_id`, `content_type`)
3. Downstream nodes can access the artifact by ID through input mappings

Artifacts are **not passed through** envelopes from node to node. They are stored in the execution context and available to any downstream node by reference. This avoids accumulation problems and keeps envelopes lightweight.

### Host Interface

The WIT interface provides nodes with:
- **Read artifact**: Fetch artifact content by ID (engine resolves from storage)
- **Write artifact**: Store content and receive an artifact ID

### Path Management

The engine manages all artifact storage and path resolution. Wasm components never see raw filesystem paths - they work with artifact IDs that the engine resolves internally. This provides:

- Clean separation of concerns
- Portability across storage backends
- Security (components can't access arbitrary paths)

### Artifact Cleanup

Artifacts are not automatically deleted. Cleanup is managed manually via CLI:

**CLI Commands:**
- Query completed workflows by time range (e.g., `fuscia workflows list --completed-before 2026-01-01`)
- Delete artifacts for a workflow (e.g., `fuscia artifacts delete --workflow-id wf_abc`)

**Separation of Concerns:**
- Deleting artifacts does **not** delete workflow execution records
- Execution history (metadata, status, timing) is preserved for querying
- Workflow records can be deleted separately if needed

**Future Automation:**
- Scheduled job queries for old completed workflows
- Pushes workflow IDs to a queue
- Worker processes queue and deletes artifacts

## Node Output Model

### Request/Response with Streaming Extensibility

Nodes use a request/response model initially: receive input, execute, return output when complete.

The output type is designed to be forward-compatible with streaming:

```wit
variant node-output {
  complete(output-data),
  // Future: stream-start(stream-handle),
  // Future: stream-chunk(stream-handle, output-data),
  // Future: stream-end(stream-handle),
}
```

For now, nodes return `complete(...)`. The engine processes this as a single response.

When streaming is added later:
1. The WIT variant is extended with stream cases
2. Existing nodes continue to work unchanged (they still return `complete`)
3. New nodes can opt into streaming

Internally, the engine handles node output as an async channel from the start, even though today it only ever yields one item. This makes streaming a non-breaking addition.

## Storage Architecture

The engine uses a pluggable storage architecture with traits abstracting each concern. This allows running locally for development or on cloud infrastructure for production.

### Storage Backends

| Concern | Trait | Local Implementation | Cloud Implementation |
|---------|-------|---------------------|---------------------|
| Persistent data | `WorkflowStore` | SQLite (sqlx) | PostgreSQL |
| Artifacts/blobs | `ArtifactStore` | Local filesystem | S3-compatible (Minio, AWS S3) |
| Cache (optional) | `CacheStore` | In-memory | Redis |

### What Goes Where

**Persistent Store (SQLite/Postgres)**
- Workflow definitions
- Workflow execution records (including active executions)
- Task records
- Node output data (stored as JSON blob)

All execution state is persisted to the database. This provides:
- Crash recovery - can resume workflows after restart
- Single source of truth - no sync issues between stores
- External visibility - can query active executions from CLI/API

**Artifact Store (Filesystem/S3)**
- All binary artifacts (stored immediately when produced)
- Available to any node in the workflow by reference

**Cache (Optional, Future)**
Redis or in-memory cache can be added later for:
- Caching frequently-read data (workflow definitions)
- Pub/sub for real-time execution updates
- Distributed locking for multiple engine instances

The cache is not required for correctness - it's purely an optimization.

### Dependency Injection

Each workflow run receives dependencies on these storage traits. The traits know how to fetch/store data from the underlying backend, and implementations can be swapped based on environment.

## Retry and Error Handling

### Retry Policies

Retry policies are configurable at two levels:

| Level | Description |
|-------|-------------|
| Workflow | Default retry policy for all nodes in the workflow |
| Node | Override policy for a specific node (completely replaces workflow default) |

Node-level settings fully override workflow defaults. If a workflow specifies 3 retries but a node specifies 5, the node gets 5 retries.

### Failure Behavior

By default, a node failure (after retries exhausted) fails the entire workflow.

Nodes can be marked as non-critical with `fail_workflow: false`. Non-critical node failures:
- Are logged
- Mark the node as failed in the execution record
- Do not block other branches from continuing

### Workflow Execution Status

| Status | Description |
|--------|-------------|
| `succeeded` | All nodes completed successfully |
| `failed` | A critical node failed (after retries exhausted) |
| `completed_with_errors` | Workflow finished but one or more non-critical nodes failed |

## Observability

The engine uses OpenTelemetry for logging, tracing, and metrics.

### Stack

| Component | Technology |
|-----------|------------|
| Rust instrumentation | `tracing` + `tracing-opentelemetry` crates |
| Trace backend | Jaeger |

### Benefits

- Structured logging with trace context (correlate logs across nodes in a workflow)
- Distributed tracing (visualize workflow execution as spans)
- Metrics (track execution times, failure rates, etc.)
- Vendor-agnostic (can switch backends without code changes)

Jaeger runs as a container and provides a web UI for visualizing traces, making it ideal for development and debugging workflow executions.

## Wasm Runtime

### Runtime Selection

**Wasmtime** is the chosen runtime because:
- Native Rust implementation
- Best WIT/component model support
- Developed by the same team (Bytecode Alliance) that drives the WIT and component model specifications

### Async Execution

The WebAssembly component model doesn't have native async support yet. WASIp3 will add first-class async with `stream` and `future` types, but it's not stable until ~November 2025.

**Current approach:** Use `tokio::task::spawn_blocking` to run Wasm execution on a blocking thread pool. The async engine spawns blocking tasks for Wasm calls.

**Future migration:** When WASIp3 stabilizes, migrate to native async component model support.

### Timeouts

Timeouts are configurable at two levels (same pattern as retries):

| Level | Description |
|-------|-------------|
| Workflow | Default timeout for all nodes |
| Node | Override timeout for a specific node (replaces workflow default) |

Timeout enforcement uses Wasmtime's epoch interruption to forcibly stop execution when the timeout is reached.

### Cancellation

Cancellation is forcible, not cooperative:
- No mechanism for nodes to check "should I stop?"
- When timeout or cancellation occurs, the Wasm execution is killed via epoch interruption
- The engine cleans up host-side resources (artifact handles, etc.)

This matches n8n's approach and keeps the implementation simple. Cooperative cancellation can be added later if needed for long-running nodes that want to checkpoint progress.

### Component Instantiation

Each task gets a **fresh Wasm instance**. Instances are not reused across tasks or executions.

**Rationale:**
- Safer - no state leakage between executions
- Simpler mental model - nodes are truly stateless
- Avoids tracking "clean" vs "dirty" instances

**Future optimization:** If instantiation overhead becomes a bottleneck (e.g., high-throughput loops), Wasmtime's instance pooling can be added to reduce cost while maintaining isolation.
