# Triggers

Triggers define how workflows are initiated. Every workflow has exactly one trigger node, which must be an entry point (no incoming edges).

## Trigger Types

| Type | Description |
|------|-------------|
| Manual | User-initiated via CLI or UI |
| Poll | Scheduled execution with configurable `interval_ms` |
| Webhook | HTTP request trigger with specified `method` |

## Built-in vs Component Triggers

Triggers have a **type** (Manual, Poll, Webhook) and an optional **component**:

- **Built-in (no component)**: Simple triggers handled by the orchestrator directly. The trigger payload is passed through as the trigger node's output.
- **Component (with wasm/lua)**: Custom validation/transformation logic executed through the `RuntimeRegistry`, using the same `NodeExecutor` path as tasks. The component can validate the incoming payload, enrich it, or reject invalid requests by returning `status: "pending"`.

## Trigger Execution

During `invoke()`, the orchestrator runs the trigger node first:

1. If no component — pass the payload through as output, continue
2. If component present:
   - Execute via `RuntimeRegistry` (same path as tasks)
   - Pass the trigger payload as input
3. Check output JSON:
   - `status: "pending"` — short-circuit, return immediately without running the graph
   - Anything else (including `status: "completed"`) — use output as trigger data, proceed with graph execution

## Output Convention

Trigger components return JSON. The orchestrator checks for a `status` field:

```json
// Continue workflow execution
{"status": "completed", "enriched": true, "data": "..."}

// Short-circuit (skip the rest of the workflow)
{"status": "pending"}
```

Any output without `status: "pending"` is treated as "continue". The full output object becomes the trigger node's output and is available to downstream nodes via template expressions.

## Unified Execution Model

Trigger components use the same `LockedComponent` type as task components and execute through the same `RuntimeRegistry`/`NodeExecutor` path. This means:

- Triggers can be written in **any supported runtime** (Wasm, Lua, etc.)
- Triggers have access to the same **host capabilities** (KV, config, log, HTTP)
- No separate trigger-specific execution infrastructure is needed

## Validation

The orchestrator validates:
- Exactly one trigger node per workflow
- All entry points (no incoming edges) are trigger nodes
- Non-trigger nodes with no incoming edges are rejected as orphans
- Trigger component references resolve to valid installed components
