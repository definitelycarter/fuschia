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
- `wit/` - WebAssembly Interface Type (WIT) definitions
  - `task.wit` - Task interface with context (execution-id, node-id, task-id) and execute function
  - `trigger.wit` - Trigger interface with event variants (poll, webhook via WASI incoming-request) and handle function returning status (completed/pending)
  - `kv.wit` - Key-value store host import for component state persistence
  - `config.wit` - Config host import for lazy configuration lookup
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
