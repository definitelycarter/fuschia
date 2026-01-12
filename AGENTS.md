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
  - `fuscia-component` - Component registry system. Manages installed Wasm components with manifests (name, version, description, digest, input schema). Components are stored in `~/.fuscia/components/` with a npm-like directory structure.
  - `fuscia-resolver` - Transforms `WorkflowDef` (config) into `Workflow` (locked). Validates graph is a DAG (no cycles), resolves component references via the registry, and handles recursive loop node resolution.
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
