# Fuscia TODO

## Crates - Completed

| Crate | Description | Status |
|-------|-------------|--------|
| `fuscia-config` | Serializable workflow configuration (JSON/YAML) | Done |
| `fuscia-artifact` | Artifact storage trait + FsStore implementation | Done |
| `fuscia-store` | Workflow execution/task storage trait + SqliteStore | Done |
| `fuscia-workflow` | Locked/resolved workflow with graph traversal | Done |

## Crates - Outstanding

| Crate | Description | Priority |
|-------|-------------|----------|
| `fuscia-component-cache` | Content-addressable cache for Wasm components (by digest) | High |
| `fuscia-resolver` | Convert `WorkflowDef` (config) → `Workflow` (locked), fetch and hash components | High |
| `fuscia-runtime` | Execution state types (step inputs, results, transient state) | High |
| `fuscia-component` | Wasmtime integration, load and execute Wasm components | High |
| `fuscia-engine` | Workflow orchestration, scheduling, graph execution | High |
| `fuscia-cli` | Command-line interface | Medium |

## Features - Outstanding

| Feature | Description | Notes |
|---------|-------------|-------|
| Config → Workflow locking | Validate graph, resolve component sources, compute digests | Needs `fuscia-resolver` |
| Component caching | Fetch from URL/file, store by digest | Needs `fuscia-component-cache` |
| Wasm execution | Load component from cache, call via WIT | Needs `fuscia-component` |
| Input path resolution | Resolve `$.node.output.field` expressions at runtime | Needs `fuscia-runtime` or engine |
| Parallel branch execution | Spawn concurrent tasks for independent branches | Needs `fuscia-engine` |
| Join node handling | Wait for branches, apply strategy (All/Any) | Needs `fuscia-engine` |
| Loop execution | Iterate over collection, execute nested workflow | Needs `fuscia-engine` |
| Retry logic | Retry failed nodes per policy | Needs `fuscia-engine` |
| Timeout enforcement | Kill Wasm execution via epoch interruption | Needs `fuscia-component` |
| Observability | OpenTelemetry tracing to Jaeger | Needs integration across crates |

## Open Questions

- Should `fuscia-resolver` and `fuscia-component-cache` be separate crates or combined?
- Where does input path expression parsing live? (fuscia-workflow or fuscia-runtime)
- WIT interface design for components (deferred for now)
