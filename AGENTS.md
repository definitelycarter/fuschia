# Fuschia

Fuschia is a workflow engine similar to n8n, built on WebAssembly components using WIT (WebAssembly Interface Types). Each workflow node is a Wasm component with explicitly defined capabilities.

## Documentation

- [docs/DESIGN.md](./docs/DESIGN.md) - Architecture and design decisions
- [docs/USE_CASES.md](./docs/USE_CASES.md) - Example workflows with diagrams and JSON definitions
- [TODO.md](./TODO.md) - Outstanding crates and features

## Project Structure

- `src/` - Main library crate
- `crates/` - Workspace member crates
  - `fuschia-config` - Serializable workflow configuration types (workflow definitions, nodes, edges). Represents workflows before they are loaded and resolved by the engine. Can be deserialized from JSON files or database storage.
  - `fuschia-artifact` - Artifact storage trait and implementations. Provides async streaming interface for storing/retrieving binary artifacts. Includes `FsStore` for local filesystem storage.
  - `fuschia-workflow` - Locked/resolved workflow representation. Validated graph structure with components referenced by name, version, and digest. Ready for execution. Includes graph traversal utilities.
  - `fuschia-component-registry` - Component registry system. Manages installed Wasm components with manifests containing name, version, description, digest, capabilities (allowed_hosts, allowed_paths), and HashMaps of exported tasks and triggers. Components are stored in `~/.fuschia/components/` with a npm-like directory structure.
  - `fuschia-resolver` - Transforms `WorkflowDef` (config) into `Workflow` (locked). Validates graph is a DAG (no cycles), resolves component references via the registry, and handles recursive loop node resolution.
  - `fuschia-task` - Task execution types. Defines `Task` enum (Http, Component), `TaskContext` with pre-resolved inputs (execution_id, node_id, task_id, inputs), and `TaskOutput` with output JSON and artifact references. Includes HTTP executor implementation.
  - `fuschia-trigger` - Trigger types for workflow initiation. Defines `Trigger` enum (Manual, Component) and `TriggerEvent` for starting workflow executions. Trigger type (poll/webhook) and config (interval, method) are defined in the component manifest.
  - `fuschia-host` - Shared host state for Wasm component execution. Provides `HostState<K>` with execution context (execution_id, node_id), KV store trait, and config access. Used by both task and trigger hosts.
  - `fuschia-task-host` - Task component execution. Loads, instantiates, and executes task Wasm components. Implements host imports (kv, config, log) via `TaskHostState`. Supports epoch-based timeouts.
  - `fuschia-trigger-host` - Trigger component execution. Loads and executes trigger Wasm components. Implements host imports via `TriggerHostState`. Returns trigger status (completed/pending).
  - `fuschia-runtime` - Workflow runtime. Provides `Runtime` with `invoke(payload, cancel)` for full workflow execution and `invoke_node(node_id, payload, cancel)` for single-node debugging. Executes trigger wasm components first (short-circuits on `Pending`, continues on `Completed`). Handles graph traversal, parallel scheduling, minijinja-based input resolution, type coercion, component caching, and wasm component execution via `TaskExecutor` and `TriggerExecutor`. Enforces single trigger per workflow. Supports cancellation via `CancellationToken`.
  - `fuschia-engine` - Workflow execution engine. Provides `WorkflowRunner` for channel-based triggering (daemon mode). Owns an `Arc<Runtime>` and calls `runtime.invoke()` when payloads arrive. Re-exports key types from `fuschia-runtime`.
  - `fuschia-world` - Wasmtime bindgen host world. Uses `wasmtime::component::bindgen!` to generate Rust bindings from WIT interfaces. Defines the host world that combines all imports (kv, config, log, http) and exports (task, trigger).
- `wit/` - WebAssembly Interface Type (WIT) definitions
  - `world.wit` - Platform world with shared imports, plus task-component and trigger-component worlds
  - `deps/` - WIT package dependencies
    - `fuschia-task/task.wit` - Task interface with context (execution-id, node-id, task-id) and execute function
    - `fuschia-trigger/trigger.wit` - Trigger interface with event variants (poll, webhook via WASI incoming-request) and handle function returning status (completed/pending)
    - `fuschia-kv/kv.wit` - Key-value store host import for component state persistence
    - `fuschia-config/config.wit` - Config host import for lazy configuration lookup
    - `fuschia-log/log.wit` - Logging interface that routes to OpenTelemetry on the host
    - `wasi_http@0.2.0.wit` - WASI HTTP types and outgoing handler
    - `wasi_io@0.2.0.wit`, `wasi_clocks@0.2.0.wit`, etc. - WASI dependencies
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
- Do not automatically commit or push to this repository - wait for explicit user approval
- Avoid `clone()` in production code - provide justification if proposing it (acceptable in tests)
- Avoid `unwrap()`, `expect()`, and other panic-prone error handling in production code (acceptable in tests)
