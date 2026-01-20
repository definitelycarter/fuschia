# Examples

This directory contains example workflows and pre-built components for testing the fuschia CLI.

## Directory Structure

```
examples/
├── README.md
├── simple-workflow.json      # A simple workflow with one task
└── components/               # Pre-built components for examples
    └── test-task--1.0.0/     # Format: {name}--{version}/
        ├── component.wasm
        └── manifest.json
```

## Running the Examples

### Run a workflow

```bash
echo '{"message": "hello world"}' | cargo run -- --data-dir examples run workflow examples/simple-workflow.json
```

### Run a single node

```bash
echo '{"message": "direct execution"}' | cargo run -- --data-dir examples run node examples/simple-workflow.json process
```

### Without piping (empty payload)

```bash
cargo run -- --data-dir examples run workflow examples/simple-workflow.json
```

## Rebuilding Components

The components in `examples/components/` are pre-built for convenience. To rebuild them:

```bash
cd test-components/test-task-component
cargo component build --release
cp target/wasm32-wasip1/release/test_task_component.wasm \
   ../../examples/components/test-task--1.0.0/component.wasm
```
