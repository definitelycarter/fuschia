# Fuscia

A workflow engine built on WebAssembly components.

## Quick Start

### Run a workflow

```bash
echo '{"message": "hello world"}' | cargo run -- run workflow examples/simple-workflow.json --data-dir examples
```

### Run a single task

```bash
echo '{"message": "test"}' | cargo run -- run task examples/simple-workflow.json --node process --data-dir examples
```

### CLI Help

```bash
cargo run -- --help
cargo run -- run --help
cargo run -- run workflow --help
```

## Examples

See the [examples directory](./examples/README.md) for sample workflows and pre-built components.

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test --workspace
```

### Build test components

```bash
cd test-components/test-task-component
cargo component build --release
```
