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

## Open Questions

- Where does input path expression parsing live? (fuscia-workflow or fuscia-runtime)
- WIT interface design for components (deferred for now)
- Should `Graph::downstream`/`upstream` return `Option<&[String]>` instead of `&[]` for missing nodes?
