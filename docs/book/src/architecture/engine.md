# Orchestrator

The orchestrator (`fuschia-workflow-orchestrator`) owns a workflow graph and walks it wave-by-wave, resolving inputs and dispatching node execution to runtime backends. It has no knowledge of wasmtime, Lua, or any specific VM.

## Responsibilities

- **Graph traversal** — Find ready nodes (all upstream dependencies complete), execute them, repeat
- **Parallel scheduling** — Independent branches spawn as concurrent tokio tasks
- **Input resolution** — Render minijinja templates against upstream data, coerce types per schema
- **Runtime dispatch** — Hand off node execution to a `NodeExecutor` implementation via `RuntimeRegistry`
- **Trigger execution** — Run the trigger node first; if the trigger component returns `status: "pending"`, short-circuit
- **Cancellation** — Propagate `CancellationToken` to all running nodes
- **Validation** — Enforce exactly one trigger, no orphan nodes

## Execution Flow

```
invoke(payload, cancel)
  │
  ├─ validate_workflow()
  │    • exactly one trigger node
  │    • all entry points are triggers
  │    • no orphan non-trigger nodes
  │
  ├─ execute_trigger(payload)
  │    • run trigger component via RuntimeRegistry if present
  │    • check output: status == "pending" → short-circuit
  │    • otherwise → continue with output as trigger data
  │
  └─ run_execution_loop()
       │
       loop:
         ├─ find_ready_nodes(completed)
         │    • nodes where all upstream are in completed map
         │
         ├─ execute_ready_nodes(ready)
         │    for each ready node:
         │      ├─ gather upstream data
         │      ├─ resolve_inputs() — minijinja templates → strings
         │      ├─ coerce_inputs() — strings → typed JSON per schema
         │      └─ spawn tokio task → registry.execute(rt, bytes, caps, input)
         │
         ├─ await all tasks (or cancellation)
         │
         └─ store results, find next wave
```

## What Lives in the Orchestrator vs the Runtime

| Concern | Orchestrator | Runtime |
|---------|-------------|---------|
| Graph traversal | Yes | No |
| Input resolution (minijinja) | Yes | No |
| Type coercion (schema-based) | Yes | No |
| Parallel scheduling | Yes | No |
| Cancellation propagation | Yes | Receives token |
| VM instantiation | No | Yes |
| Artifact compilation/caching | No | Yes |
| Host capability wiring | No | Yes |
| Timeout enforcement | Passes timeout | Enforces it |

## CLI Integration

The CLI (`src/main.rs`) creates an `Orchestrator` directly:

```rust
let mut runtime_registry = RuntimeRegistry::new();
runtime_registry.register(RuntimeType::Wasm, Arc::new(WasmExecutor::new(config)?));

let orchestrator = Orchestrator::new(
    workflow,
    Arc::new(runtime_registry),
    component_registry,
    OrchestratorConfig::default(),
)?;

orchestrator.invoke(payload, cancel).await?;
```
