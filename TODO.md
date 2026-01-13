# Fuscia TODO

## Crates - Completed

| Crate | Description | Status |
|-------|-------------|--------|
| `fuscia-config` | Serializable workflow configuration (JSON/YAML) | Done |
| `fuscia-artifact` | Artifact storage trait + FsStore implementation | Done |
| `fuscia-store` | Workflow execution/task storage trait + SqliteStore | Done |
| `fuscia-workflow` | Locked/resolved workflow with graph traversal | Done |
| `fuscia-component` | Component manifest, registry trait, FsComponentRegistry. Components can export multiple tasks and triggers. | Done |
| `fuscia-resolver` | Convert `WorkflowDef` (config) → `Workflow` (locked), validate graph | Done |
| `fuscia-task` | Task types (Http, Component), TaskContext with pre-resolved inputs, TaskOutput with artifacts | Done |
| `fuscia-trigger` | Trigger types (Manual, Component), TriggerEvent for workflow initiation | Done |
| `fuscia-world` | Wasmtime bindgen host world, generates Rust bindings from WIT interfaces | Done |
| `fuscia-host` | Shared wasmtime infrastructure (Engine, Store setup, epoch timeout, pluggable KvStore trait) | Done |
| `fuscia-task-host` | Task component execution, binds to task-component world, implements kv/config/log host imports | Done |
| `fuscia-trigger-host` | Trigger component execution, binds to trigger-component world, implements kv/config/log host imports | Done |
| `fuscia-engine` | Workflow orchestration, graph execution, parallel scheduling, input resolution | Done |

## Crates - Outstanding

| Crate | Description | Priority |
|-------|-------------|----------|
| `fuscia-cli` | Command-line interface | Medium |

## Features - Completed

| Feature | Description | Notes |
|---------|-------------|-------|
| Config → Workflow locking | Validate graph (DAG), resolve component refs, detect cycles | `fuscia-resolver` |
| Component registry | Install/list/remove components, lookup by name+version | `fuscia-component` |
| Component manifest | Name, version, description, digest, capabilities, tasks, triggers | `fuscia-component` |
| Task types | Http and Component task variants | `fuscia-task` |
| Task context | Pre-resolved inputs passed to task execution | `fuscia-task` |
| Http executor | Built-in HTTP request executor | `fuscia-task` |
| Trigger types | Manual and Component trigger variants | `fuscia-trigger` |
| Trigger events | TriggerEvent for workflow initiation | `fuscia-trigger` |
| Component capabilities | allowed_hosts and allowed_paths for security sandboxing | `fuscia-component` |
| TriggerType in manifest | Poll (interval_ms) and Webhook (method) defined in component manifest | `fuscia-component` |
| WIT task interface | Task execute function with context (execution-id, node-id, task-id) | `wit/task.wit` |
| WIT trigger interface | Trigger handle function with event variants (poll, webhook) | `wit/trigger.wit` |
| WIT host imports | Key-value store and config imports for components | `wit/kv.wit`, `wit/config.wit` |
| WIT log interface | Logging interface that routes to OpenTelemetry | `wit/log.wit` |
| WIT platform world | Shared world with common imports (kv, config, log, http) | `wit/world.wit` |
| Host bindings generation | wasmtime bindgen generates Rust traits from WIT | `fuscia-world` |
| Pluggable KV store | Async KvStore trait with InMemoryKvStore implementation | `fuscia-host` |
| Engine configuration | Wasmtime Engine with epoch interruption and async support | `fuscia-host` |
| Host state management | HostState generic over KvStore, Store creation helpers | `fuscia-host` |
| Task component execution | Load, instantiate, execute task components via wasmtime | `fuscia-task-host` |
| Trigger component execution | Load, instantiate, execute trigger components via wasmtime | `fuscia-trigger-host` |
| WASI Preview 2 support | WasiView, ResourceTable, p2::add_to_linker_async for component WASI imports | `fuscia-task-host`, `fuscia-trigger-host` |
| Test wasm components | test-task-component and test-trigger-component for integration testing | `test-components/` |
| Integration tests | Tests for task and trigger host execution with real wasm components | `fuscia-task-host`, `fuscia-trigger-host` |
| Workflow engine | Graph traversal with parallel node execution via tokio tasks | `fuscia-engine` |
| WorkflowRunner | Channel-based workflow triggering with mpsc sender/receiver | `fuscia-engine` |
| Input resolution | Minijinja templating for node inputs (`{{ field }}`, `{{ name \| upper }}`) | `fuscia-engine` |
| Component caching | Compile wasm once, instantiate on-demand per execution | `fuscia-engine` |
| Cancellation support | CancellationToken for workflow-level and epoch interruption for node-level | `fuscia-engine` |
| Parallel branch execution | Spawn concurrent tokio tasks for independent graph branches | `fuscia-engine` |
| NodeType::Trigger | Proper trigger node type with TriggerType (Manual, Poll, Webhook) | `fuscia-config`, `fuscia-workflow` |
| TriggerComponentRef | Type-safe pairing of component + trigger_name for trigger nodes | `fuscia-config` |
| LockedTriggerComponent | Locked trigger component with digest and trigger export name | `fuscia-workflow` |
| Orphan node detection | Validation that non-trigger nodes must have incoming edges | `fuscia-engine` |
| Input type coercion | Parse resolved template strings to typed JSON values (string, number, boolean, etc.) | `fuscia-engine` |
| Simplified InputValue | InputValue is now a type alias for String (all inputs are templates) | `fuscia-config` |
| Schema in LockedComponent | `input_schema` copied from manifest at lock time for self-contained execution | `fuscia-workflow`, `fuscia-resolver` |
| Task/trigger name in locked types | `task_name` and `trigger_name` identify which export to use from component | `fuscia-config`, `fuscia-workflow` |

## Features - Outstanding

| Feature | Description | Notes |
|---------|-------------|-------|
| HTTP outbound for components | Add `wasi:http/outgoing-handler` to platform world | Requires wasmtime-wasi-http integration |
| Join node handling | Wait for branches, apply strategy (All/Any) | Needs `fuscia-engine` enhancement |
| Loop execution | Iterate over collection, execute nested workflow | Needs `fuscia-engine` enhancement |
| Retry logic | Retry failed nodes per policy | Needs `fuscia-engine` enhancement |
| Observability | OpenTelemetry tracing to Jaeger | Needs integration across crates |
| Component packaging | Bundle manifest + wasm + readme + assets into .fcpkg | Needs `fuscia-cli` |
| Store integration | Persist execution records to fuscia-store | Needs `fuscia-engine` enhancement |

## Gaps - Existing Crates

### fuscia-config

| Gap | Description | Priority |
|-----|-------------|----------|
| Missing retry fields on NodeDef | `retry_backoff` and `retry_initial_delay_ms` only on `WorkflowDef`, not `NodeDef` | Medium |

### fuscia-workflow

| Gap | Description | Priority |
|-----|-------------|----------|
| Graph methods return `&[]` for missing nodes | `downstream()`/`upstream()` can't distinguish "no edges" vs "node doesn't exist" | Medium |

### fuscia-resolver

| Gap | Description | Priority |
|-----|-------------|----------|
| Join node validation | Doesn't validate that `Join` nodes actually have multiple incoming edges | Medium |

### fuscia-engine

| Gap | Description | Priority |
|-----|-------------|----------|
| Required field validation | No validation that required input fields are present | Medium |

### fuscia-component

| Gap | Description | Priority |
|-----|-------------|----------|
| Digest verification | Digest in manifest never verified against actual wasm binary | Medium |

### fuscia-artifact

| Gap | Description | Priority |
|-----|-------------|----------|
| Content-type not persisted | `FsStore` accepts `content_type` but doesn't store it anywhere | Medium |

### fuscia-store

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
| `fuscia-host` | Shared wasmtime infrastructure: Engine configuration, Store setup, epoch-based timeout utilities, common host import implementations (kv, config, log) |
| `fuscia-task-host` | Task-specific execution. Imports `fuscia-host`, binds to `task-component` world, invokes `task.execute` |
| `fuscia-trigger-host` | Trigger-specific execution. Imports `fuscia-host`, binds to `trigger-component` world, invokes `trigger.handle` |

**Rationale:** Clean separation - `fuscia-host` provides wasmtime foundation without knowing about tasks/triggers. Specialized crates compose it with their WIT bindings.

### Workflow Engine Architecture

The engine layer is split into runner and engine:

| Type | Responsibility |
|------|----------------|
| `WorkflowRunner` | Owns mpsc channel (sender + receiver), provides `run(payload)` for direct triggering, `sender()` for webhook/poll handlers, `start(cancel)` for execution loop |
| `WorkflowEngine` | Executes workflow given payload, handles graph traversal, parallel scheduling, input resolution via minijinja |

**Rationale:** Separation between triggering mechanism (Runner) and execution logic (Engine). Triggers are decoupled from workflow execution.

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

**Note:** `ComponentError` doesn't automatically mean task failure. The engine decides based on workflow config whether it's "failed" or "completed with errors".

Workflow execution errors:

```rust
enum ExecutionError {
    /// Input template resolution failed
    InputResolution { node_id: String, message: String },
    /// Component execution failed
    ComponentExecution { node_id: String, source: HostError },
    /// Workflow was cancelled
    Cancelled,
    /// Node execution timed out
    Timeout { node_id: String },
}
```

## Open Questions

| Question | Context |
|----------|---------|
| HTTP outbound filtering | How to enforce `allowed_hosts` with `wasmtime-wasi-http`? Custom `WasiHttpView` wrapper or implement own handler? |
| Shared Engine ownership | Should `fuscia-host` own the Engine singleton, or should caller create and pass it in? |
| Path expression parsing location | Should live in `fuscia-config` (parse at config time) or `fuscia-task` (parse at execution)? |
| Graph method return types | Should `downstream()`/`upstream()` return `Option<&[String]>` instead of `&[]`? |
| Loop item injection | How does `{ "item": {...}, "index": 0 }` get passed to nested workflow inputs? |
| Join node output shape | What's the output - aggregated map of branch outputs? Pass-through? |
| WorkflowExecution.config type | Should store original `WorkflowDef` or locked `Workflow` for audit trail? |
| KV store value types | Should kv.wit support complex types (json, number, bool, object) or just strings? |
| WorkflowRunner naming | Current name may be confusing - reconsider naming |
