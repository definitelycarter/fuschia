# Fuscia TODO

## Crates - Completed

| Crate | Description | Status |
|-------|-------------|--------|
| `fuscia-config` | Serializable workflow configuration (JSON/YAML) | Done |
| `fuscia-artifact` | Artifact storage trait + FsStore implementation | Done |
| `fuscia-store` | Workflow execution/task storage trait + SqliteStore | Done |
| `fuscia-workflow` | Locked/resolved workflow with graph traversal | Done |
| `fuscia-component` | Component manifest, registry trait, and FsComponentRegistry | Done |
| `fuscia-resolver` | Convert `WorkflowDef` (config) → `Workflow` (locked), validate graph | Done |

## Crates - Outstanding

| Crate | Description | Priority |
|-------|-------------|----------|
| `fuscia-runtime` | Execution state types (step inputs, results, transient state) | High |
| `fuscia-wasm` | Wasmtime integration, load and execute Wasm components | High |
| `fuscia-engine` | Workflow orchestration, scheduling, graph execution | High |
| `fuscia-cli` | Command-line interface | Medium |

## Features - Completed

| Feature | Description | Notes |
|---------|-------------|-------|
| Config → Workflow locking | Validate graph (DAG), resolve component refs, detect cycles | `fuscia-resolver` |
| Component registry | Install/list/remove components, lookup by name+version | `fuscia-component` |
| Component manifest | Name, version, description, digest, input schema | `fuscia-component` |

## Features - Outstanding

| Feature | Description | Notes |
|---------|-------------|-------|
| Wasm execution | Load component from registry, call via WIT | Needs `fuscia-wasm` |
| Input path resolution | Resolve `$.node.output.field` expressions at runtime | Needs `fuscia-runtime` or engine |
| Parallel branch execution | Spawn concurrent tasks for independent branches | Needs `fuscia-engine` |
| Join node handling | Wait for branches, apply strategy (All/Any) | Needs `fuscia-engine` |
| Loop execution | Iterate over collection, execute nested workflow | Needs `fuscia-engine` |
| Retry logic | Retry failed nodes per policy | Needs `fuscia-engine` |
| Timeout enforcement | Kill Wasm execution via epoch interruption | Needs `fuscia-wasm` |
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

## Open Questions

| Question | Context |
|----------|---------|
| Path expression parsing location | Should live in `fuscia-config` (parse at config time) or `fuscia-runtime` (parse at execution)? |
| WIT interface design | Deferred - need to design component interface for Wasm modules |
| Graph method return types | Should `downstream()`/`upstream()` return `Option<&[String]>` instead of `&[]`? |
| Loop item injection | How does `{ "item": {...}, "index": 0 }` get passed to nested workflow inputs? |
| Join node output shape | What's the output - aggregated map of branch outputs? Pass-through? |
| WorkflowExecution.config type | Should store original `WorkflowDef` or locked `Workflow` for audit trail? |
