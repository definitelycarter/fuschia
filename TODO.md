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

## Crates - Outstanding

| Crate | Description | Priority |
|-------|-------------|----------|
| `fuscia-trigger-host` | Trigger component execution, binds to trigger-component world | High |
| `fuscia-engine` | Workflow orchestration, scheduling, graph execution | High |
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

## Features - Outstanding

| Feature | Description | Notes |
|---------|-------------|-------|
| HTTP outbound for components | Add `wasi:http/outgoing-handler` to platform world | Requires wasmtime-wasi-http integration |
| Input path resolution | Resolve `$.node.output.field` expressions at runtime | Needs `fuscia-engine` |
| Timeout enforcement | Kill Wasm execution via epoch interruption | Infrastructure in `fuscia-host`, needs integration |
| Parallel branch execution | Spawn concurrent tasks for independent branches | Needs `fuscia-engine` |
| Join node handling | Wait for branches, apply strategy (All/Any) | Needs `fuscia-engine` |
| Loop execution | Iterate over collection, execute nested workflow | Needs `fuscia-engine` |
| Retry logic | Retry failed nodes per policy | Needs `fuscia-engine` |
| Observability | OpenTelemetry tracing to Jaeger | Needs integration across crates |
| Component packaging | Bundle manifest + wasm + readme + assets into .fcpkg | Needs `fuscia-cli` |

## Gaps - Existing Crates

### fuscia-config

| Gap | Description | Priority |
|-----|-------------|----------|
| Missing retry fields on NodeDef | `retry_backoff` and `retry_initial_delay_ms` only on `WorkflowDef`, not `NodeDef` | Medium |
| Path expression validation | `InputValue::Path` is a raw `String` with no validation of `$.node.output.field` syntax | High |

### fuscia-workflow

| Gap | Description | Priority |
|-----|-------------|----------|
| Graph methods return `&[]` for missing nodes | `downstream()`/`upstream()` can't distinguish "no edges" vs "node doesn't exist" | Medium |

### fuscia-resolver

| Gap | Description | Priority |
|-----|-------------|----------|
| Input path validation | Doesn't validate that `InputValue::Path` references point to valid node IDs | High |
| Join node validation | Doesn't validate that `Join` nodes actually have multiple incoming edges | Medium |

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
