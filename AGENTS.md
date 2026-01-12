# Fuscia

Fuscia is a workflow engine similar to n8n, built on WebAssembly components using WIT (WebAssembly Interface Types). Each workflow node is a Wasm component with explicitly defined capabilities.

## Documentation

- [docs/DESIGN.md](./docs/DESIGN.md) - Architecture and design decisions
- [docs/USE_CASES.md](./docs/USE_CASES.md) - Example workflows with diagrams and JSON definitions
- [TODO.md](./TODO.md) - Outstanding crates and features

## Project Structure

- `src/` - Main library crate
- `crates/` - Workspace member crates
  - `fuscia-config` - Serializable workflow configuration types (workflow definitions, nodes, edges). Represents workflows before they are loaded and resolved by the engine. Can be deserialized from JSON files or database storage.
  - `fuscia-artifact` - Artifact storage trait and implementations. Provides async streaming interface for storing/retrieving binary artifacts. Includes `FsStore` for local filesystem storage.
  - `fuscia-store` - Workflow execution and task storage trait and implementations. Stores execution state, task records, and snapshots of workflow configs. Includes `SqliteStore` implementation.
  - `fuscia-workflow` - Locked/resolved workflow representation. Validated graph structure with components referenced by name, version, and digest. Ready for execution. Includes graph traversal utilities.
  - `fuscia-component` - Component registry system. Manages installed Wasm components with manifests containing name, version, description, digest, capabilities (allowed_hosts, allowed_paths), and HashMaps of exported tasks and triggers. Components are stored in `~/.fuscia/components/` with a npm-like directory structure.
  - `fuscia-resolver` - Transforms `WorkflowDef` (config) into `Workflow` (locked). Validates graph is a DAG (no cycles), resolves component references via the registry, and handles recursive loop node resolution.
  - `fuscia-task` - Task execution types. Defines `Task` enum (Http, Component), `TaskContext` with pre-resolved inputs (execution_id, node_id, task_id, inputs), and `TaskOutput` with output JSON and artifact references. Includes HTTP executor implementation.
  - `fuscia-trigger` - Trigger types for workflow initiation. Defines `Trigger` enum (Manual, Component) and `TriggerEvent` for starting workflow executions. Trigger type (poll/webhook) and config (interval, method) are defined in the component manifest.
  - `fuscia-host` - Shared host state for Wasm component execution. Provides `HostState<K>` with execution context (execution_id, node_id), KV store trait, and config access. Used by both task and trigger hosts.
  - `fuscia-task-host` - Task component execution. Loads, instantiates, and executes task Wasm components. Implements host imports (kv, config, log) via `TaskHostState`. Supports epoch-based timeouts.
  - `fuscia-trigger-host` - Trigger component execution. Loads and executes trigger Wasm components. Implements host imports via `TriggerHostState`. Returns trigger status (completed/pending).
  - `fuscia-engine` - Workflow execution engine. Contains `WorkflowEngine` for graph traversal and task execution, `WorkflowRunner` for channel-based triggering, and minijinja-based input resolution. Executes nodes in parallel when dependencies are satisfied. Supports cancellation via `CancellationToken`.
  - `fuscia-world` - Wasmtime bindgen host world. Uses `wasmtime::component::bindgen!` to generate Rust bindings from WIT interfaces. Defines the host world that combines all imports (kv, config, log, http) and exports (task, trigger).
- `wit/` - WebAssembly Interface Type (WIT) definitions
  - `world.wit` - Platform world with shared imports, plus task-component and trigger-component worlds
  - `deps/` - WIT package dependencies
    - `fuscia-task/task.wit` - Task interface with context (execution-id, node-id, task-id) and execute function
    - `fuscia-trigger/trigger.wit` - Trigger interface with event variants (poll, webhook via WASI incoming-request) and handle function returning status (completed/pending)
    - `fuscia-kv/kv.wit` - Key-value store host import for component state persistence
    - `fuscia-config/config.wit` - Config host import for lazy configuration lookup
    - `fuscia-log/log.wit` - Logging interface that routes to OpenTelemetry on the host
    - `wasi_http@0.2.0.wit` - WASI HTTP types and outgoing handler
    - `wasi_io@0.2.0.wit`, `wasi_clocks@0.2.0.wit`, etc. - WASI dependencies
- `migrations/` - SQLite database migrations (sqlx)
- `docs/` - Design documentation

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Formatting

```bash
cargo fmt
```

## Guidelines

- Follow Rust idioms and best practices
- Use `cargo fmt` before committing
- Ensure all tests pass with `cargo test`
- Add tests for new functionality
