# Fuschia TODO

## Crates - Completed

| Crate | Description | Status |
|-------|-------------|--------|
| `fuschia-config` | Serializable workflow configuration (JSON/YAML) | Done |
| `fuschia-artifact` | Artifact storage trait + FsStore implementation | Done |
| `fuschia-store` | Workflow execution/task storage trait + SqliteStore | Done |
| `fuschia-workflow` | Locked/resolved workflow with graph traversal | Done |
| `fuschia-component-registry` | Component manifest, registry trait, FsComponentRegistry. Components can export multiple tasks and triggers. | Done |
| `fuschia-resolver` | Convert `WorkflowDef` (config) → `Workflow` (locked), validate graph | Done |
| `fuschia-task` | Task types (Http, Component), TaskContext with pre-resolved inputs, TaskOutput with artifacts | Done |
| `fuschia-trigger` | Trigger types (Manual, Component), TriggerEvent for workflow initiation | Done |
| `fuschia-world` | Wasmtime bindgen host world, generates Rust bindings from WIT interfaces | Done |
| `fuschia-host` | Shared wasmtime infrastructure (Engine, Store setup, epoch timeout, pluggable KvStore trait) | Done |
| `fuschia-task-host` | Task component execution, binds to task-component world, implements kv/config/log host imports | Done |
| `fuschia-trigger-host` | Trigger component execution, binds to trigger-component world, implements kv/config/log host imports | Done |
| `fuschia-runtime` | Workflow runtime. `invoke(payload, cancel)` for full workflow execution, `invoke_node(node_id, payload, cancel)` for single-node debugging. Trigger execution, graph traversal, parallel scheduling, input resolution, type coercion, component caching. | Done |
| `fuschia-engine` | Daemon/trigger layer. `WorkflowRunner` owns `Arc<Runtime>`, channel-based triggering. Re-exports key types from `fuschia-runtime`. | Done |

## Crates - Outstanding

| Crate | Description | Priority |
|-------|-------------|----------|
| `fuschia-cli` | Command-line interface | Medium |

## Features - Completed

| Feature | Description | Notes |
|---------|-------------|-------|
| Config → Workflow locking | Validate graph (DAG), resolve component refs, detect cycles | `fuschia-resolver` |
| Component registry | Install/list/remove components, lookup by name+version | `fuschia-component-registry` |
| Component manifest | Name, version, description, digest, capabilities, tasks, triggers | `fuschia-component-registry` |
| Task types | Http and Component task variants | `fuschia-task` |
| Task context | Pre-resolved inputs passed to task execution | `fuschia-task` |
| Http executor | Built-in HTTP request executor | `fuschia-task` |
| Trigger types | Manual and Component trigger variants | `fuschia-trigger` |
| Trigger events | TriggerEvent for workflow initiation | `fuschia-trigger` |
| Component capabilities | allowed_hosts and allowed_paths for security sandboxing | `fuschia-component-registry` |
| TriggerType in manifest | Poll (interval_ms) and Webhook (method) defined in component manifest | `fuschia-component-registry` |
| WIT task interface | Task execute function with context (execution-id, node-id, task-id) | `wit/task.wit` |
| WIT trigger interface | Trigger handle function with event variants (poll, webhook) | `wit/trigger.wit` |
| WIT host imports | Key-value store and config imports for components | `wit/kv.wit`, `wit/config.wit` |
| WIT log interface | Logging interface that routes to OpenTelemetry | `wit/log.wit` |
| WIT platform world | Shared world with common imports (kv, config, log, http) | `wit/world.wit` |
| Host bindings generation | wasmtime bindgen generates Rust traits from WIT | `fuschia-world` |
| Pluggable KV store | Async KvStore trait with InMemoryKvStore implementation | `fuschia-host` |
| Engine configuration | Wasmtime Engine with epoch interruption and async support | `fuschia-host` |
| Host state management | HostState generic over KvStore, Store creation helpers | `fuschia-host` |
| Task component execution | Load, instantiate, execute task components via wasmtime | `fuschia-task-host` |
| Trigger component execution | Load, instantiate, execute trigger components via wasmtime | `fuschia-trigger-host` |
| WASI Preview 2 support | WasiView, ResourceTable, p2::add_to_linker_async for component WASI imports | `fuschia-task-host`, `fuschia-trigger-host` |
| Test wasm components | test-task-component and test-trigger-component for integration testing | `test-components/` |
| Integration tests | Tests for task and trigger host execution with real wasm components | `fuschia-task-host`, `fuschia-trigger-host` |
| Workflow runtime | `Runtime` with `invoke(payload, cancel)` and `invoke_node(node_id, payload, cancel)` | `fuschia-runtime` |
| Trigger execution | Execute trigger wasm component during `invoke`, short-circuit on `Pending` | `fuschia-runtime` |
| Single trigger validation | Workflows must have exactly one trigger node | `fuschia-runtime` |
| Graph traversal | Wave-based parallel node execution via tokio tasks | `fuschia-runtime` |
| WorkflowRunner | Channel-based workflow triggering with mpsc sender/receiver, owns `Arc<Runtime>` | `fuschia-engine` |
| Input resolution | Minijinja templating for node inputs (`{{ field }}`, `{{ name \| upper }}`) | `fuschia-runtime` |
| Component caching | Compile wasm once, instantiate on-demand per execution | `fuschia-runtime` |
| Cancellation support | CancellationToken for workflow-level and epoch interruption for node-level | `fuschia-runtime` |
| Parallel branch execution | Spawn concurrent tokio tasks for independent graph branches | `fuschia-runtime` |
| NodeType::Trigger | Proper trigger node type with TriggerType (Manual, Poll, Webhook) | `fuschia-config`, `fuschia-workflow` |
| TriggerComponentRef | Type-safe pairing of component + trigger_name for trigger nodes | `fuschia-config` |
| LockedTriggerComponent | Locked trigger component with digest and trigger export name | `fuschia-workflow` |
| Orphan node detection | Validation that non-trigger nodes must have incoming edges | `fuschia-runtime` |
| Input type coercion | Parse resolved template strings to typed JSON values (string, number, boolean, etc.) | `fuschia-runtime` |
| Simplified InputValue | InputValue is now a type alias for String (all inputs are templates) | `fuschia-config` |
| Schema in LockedComponent | `input_schema` copied from manifest at lock time for self-contained execution | `fuschia-workflow`, `fuschia-resolver` |
| Task/trigger name in locked types | `task_name` and `trigger_name` identify which export to use from component | `fuschia-config`, `fuschia-workflow` |

## Features - Outstanding

| Feature | Description | Notes |
|---------|-------------|-------|
| HTTP outbound for components | Add `wasi:http/outgoing-handler` to platform world | Requires wasmtime-wasi-http integration |
| Join node handling | Wait for branches, apply strategy (All/Any) | Needs `fuschia-runtime` enhancement |
| Loop execution | Iterate over collection, execute nested workflow | Needs `fuschia-runtime` enhancement |
| Retry logic | Retry failed nodes per policy | Needs `fuschia-runtime` enhancement |
| Observability | OpenTelemetry tracing to Jaeger | Needs integration across crates |
| Component packaging | Bundle manifest + wasm + readme + assets into .fcpkg | Needs `fuschia-cli` |
| Store integration | Persist execution records to fuschia-store | Needs `fuschia-engine` enhancement |

## Gaps - Existing Crates

### fuschia-config

| Gap | Description | Priority |
|-----|-------------|----------|
| Missing retry fields on NodeDef | `retry_backoff` and `retry_initial_delay_ms` only on `WorkflowDef`, not `NodeDef` | Medium |

### fuschia-workflow

| Gap | Description | Priority |
|-----|-------------|----------|
| Graph methods return `&[]` for missing nodes | `downstream()`/`upstream()` can't distinguish "no edges" vs "node doesn't exist" | Medium |

### fuschia-resolver

| Gap | Description | Priority |
|-----|-------------|----------|
| Join node validation | Doesn't validate that `Join` nodes actually have multiple incoming edges | Medium |

### fuschia-runtime

| Gap | Description | Priority |
|-----|-------------|----------|
| Required field validation | No validation that required input fields are present | Medium |
| Digest verification | Verify component wasm SHA-256 against `LockedComponent.digest` at load time | High |

### fuschia-component

| Gap | Description | Priority |
|-----|-------------|----------|
| Digest verification | Digest in manifest never verified against actual wasm binary | Medium |

### fuschia-artifact

| Gap | Description | Priority |
|-----|-------------|----------|
| Content-type not persisted | `FsStore` accepts `content_type` but doesn't store it anywhere | Medium |

### fuschia-store

| Gap | Description | Priority |
|-----|-------------|----------|
| Missing artifacts table | No database table to track artifact metadata (id, execution_id, node_id, content_type) | High |
| Missing artifact refs in Task | `Task` type has no field to store artifact references produced by the node | High |
| Incomplete ExecutionStatus | Missing `Cancelled`, `Paused`, `TimedOut` states | Low |

## Design Decisions

### Host Crate Architecture

The wasmtime hosting layer is split into three crates:

| Crate | Responsibility |
|-------|----------------|
| `fuschia-host` | Shared wasmtime infrastructure: Engine configuration, Store setup, epoch-based timeout utilities, common host import implementations (kv, config, log) |
| `fuschia-task-host` | Task-specific execution. Imports `fuschia-host`, binds to `task-component` world, invokes `task.execute` |
| `fuschia-trigger-host` | Trigger-specific execution. Imports `fuschia-host`, binds to `trigger-component` world, invokes `trigger.handle` |

**Rationale:** Clean separation - `fuschia-host` provides wasmtime foundation without knowing about tasks/triggers. Specialized crates compose it with their WIT bindings.

### Workflow Engine Architecture

The execution layer is split into runtime and runner:

| Type | Responsibility |
|------|----------------|
| `Runtime` | Executes workflow given payload. `invoke(payload, cancel)` for full workflow execution, `invoke_node(node_id, payload, cancel)` for single-node debugging. Handles trigger execution, graph traversal, parallel scheduling, input resolution via minijinja, type coercion, and component caching. |
| `WorkflowRunner` | Daemon/trigger layer. Owns `Arc<Runtime>` and mpsc channel (sender + receiver). Provides `run(payload)` for direct triggering, `sender()` for webhook/poll handlers, `start(cancel)` for execution loop. |

**Rationale:** The runtime is the single crate responsible for workflow execution. The CLI can use the runtime directly for one-shot modes. The runner wraps the runtime for daemon/channel-based triggering.

### Input Resolution

Node inputs use **minijinja templating** for dynamic value resolution:

- **Single upstream node:** Context is the upstream node's `data` output
  ```json
  { "recipient": "{{ email }}", "greeting": "Hello {{ name | title }}!" }
  ```

- **Join nodes (future):** Context keyed by upstream node names
  ```json
  { "user_email": "{{ fetch_user.email }}", "config": "{{ get_config.setting }}" }
  ```

**Rationale:** Minijinja provides familiar Jinja2 syntax with filters, conditionals, and safe expression evaluation.

### Data Visibility

**Strict single-hop visibility:** Each node only sees its immediate upstream node's `data` output.

- No implicit access to trigger payload from non-entry nodes
- No cross-branch data sharing without explicit join
- Future: Add `context` escape hatch for explicit cross-node data sharing

**Rationale:** Predictable data flow, easier debugging, prevents implicit coupling between distant nodes.

### Trigger Node Types

Triggers have a **type** (Manual, Poll, Webhook) and optional **component**:

| Trigger Type | Description |
|--------------|-------------|
| Manual | User-initiated via CLI or UI |
| Poll | Scheduled execution with `interval_ms` |
| Webhook | HTTP request with specified `method` |

**Built-in vs Component triggers:**
- Built-in (no component): Simple triggers handled by engine directly
- Component (with wasm): Custom validation/transformation logic before workflow execution

**Type-safe pairing:** `TriggerComponentRef` and `LockedTriggerComponent` ensure `component` and `trigger_name` are always specified together.

### Concurrency Model

- **Tokio tasks for parallelism** - Ready nodes spawn as concurrent tokio tasks
- **Parallelism emerges from graph structure** - Independent branches execute in parallel automatically
- **CancellationToken per workflow execution** - Propagates cancellation to all running nodes
- **Epoch interruption for in-flight wasm** - Cancels long-running component execution

**Future:** Easy to migrate to work queue for distributed execution.

### Component Instance Lifecycle

- **Compile once, instantiate fresh per execution** - Component compilation is expensive; cache the compiled `Component`. Each task/trigger execution gets a fresh `Instance` for isolation.
- Shared `Engine` passed into host crates (Engine creation is expensive, should be shared across process).

### KV Store Scope

- **Per-execution scoping** - Each workflow execution gets isolated KV state. Components cannot see other executions' data. Host receives execution-scoped KV store trait when invoking components.

### Timeout Enforcement

- **Epoch-based interruption** via wasmtime:
  1. Engine configured with `epoch_interruption(true)`
  2. Store gets `set_epoch_deadline(1)` before calling wasm
  3. Background task increments engine epoch after timeout duration
  4. Wasm traps with `EpochInterruption` if still running
- Timeout value: `node.timeout.unwrap_or(workflow.default_timeout)` - passed as `Duration` to host crates

### Error Handling

Component execution errors are granular:

```rust
enum TaskHostError {
    /// Component instantiation failed (bad wasm, missing imports, etc.)
    Instantiation { message: String },
    /// Component trapped (OOM, stack overflow, epoch timeout, etc.)
    Trap { trap_code: Option<TrapCode>, message: String },
    /// Component returned error via WIT result (not necessarily a task failure)
    ComponentError { message: String },
}
```

**Note:** `ComponentError` doesn't automatically mean task failure. The runtime decides based on workflow config whether it's "failed" or "completed with errors".

Runtime errors:

```rust
enum RuntimeError {
    /// Execution was cancelled
    Cancelled,
    /// Failed to serialize component input
    InputSerialization { message: String },
    /// Component execution failed
    ComponentExecution { source: HostError },
    /// Failed to parse component output
    InvalidOutput { message: String },
    /// Input template resolution or type coercion failed
    InputResolution { node_id: String, message: String },
    /// Failed to load/compile a wasm component
    ComponentLoad { node_id: String, message: String },
    /// Invalid workflow graph structure
    InvalidGraph { message: String },
}
```

## Housekeeping

| Item | Description | Priority |
|------|-------------|----------|
| Fix spelling | Rename "Fuschia" → "Fuchsia" across codebase (crate names, directories, references) | Low |

## Open Questions

| Question | Context |
|----------|---------|
| HTTP outbound filtering | How to enforce `allowed_hosts` with `wasmtime-wasi-http`? Custom `WasiHttpView` wrapper or implement own handler? |
| Shared Engine ownership | Should `fuschia-host` own the Engine singleton, or should caller create and pass it in? |
| Path expression parsing location | Should live in `fuschia-config` (parse at config time) or `fuschia-task` (parse at execution)? |
| Graph method return types | Should `downstream()`/`upstream()` return `Option<&[String]>` instead of `&[]`? |
| Loop item injection | How does `{ "item": {...}, "index": 0 }` get passed to nested workflow inputs? |
| Join node output shape | What's the output - aggregated map of branch outputs? Pass-through? |
| WorkflowExecution.config type | Should store original `WorkflowDef` or locked `Workflow` for audit trail? |
| KV store value types | Should kv.wit support complex types (json, number, bool, object) or just strings? |
| WorkflowRunner naming | Current name may be confusing - reconsider naming |
| Distributed execution model | `fuschia start` daemon mode for production: init container pulls components at deploy time, pods wait for messages from broker. Each workflow node gets pre-warmed pods. Message format: `{execution_id, task_id, input}`. Orchestrator resolves templates, workers just execute. |
