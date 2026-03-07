# Component Triggers

Component triggers run custom logic to validate or transform incoming events before the workflow graph executes. They use the same execution path as task components.

## Interface

Trigger components implement the same `execute(ctx, data)` interface as tasks. The orchestrator passes the trigger payload as input and interprets the output:

- Output with `"status": "pending"` → short-circuit, don't run the workflow graph
- Any other output → continue, use as trigger node's output

## Examples

### Lua Trigger (validate and enrich)

```lua
function execute(ctx, data)
    local input = json_decode(data)

    -- Validate the request
    if not input.api_key then
        return '{"status": "pending"}'
    end

    -- Enrich and continue
    return json_encode({
        status = "completed",
        validated = true,
        user = input.user,
        timestamp = os.time()
    })
end
```

### Lua Trigger (poll with state)

```lua
function execute(ctx, data)
    -- Check for new data using KV to track state
    local last_id = ctx.kv_get("last_seen_id") or "0"

    -- In a real component, this would check an external API
    local new_id = last_id  -- placeholder

    if new_id == last_id then
        return '{"status": "pending"}'
    end

    ctx.kv_set("last_seen_id", new_id)
    return json_encode({
        status = "completed",
        new_items = {}
    })
end
```

## Event Types

### Poll

The orchestrator calls the trigger component at the configured `interval_ms`. The component checks for new data and returns:

- `status: "completed"` with data — new data found, start workflow with this payload
- `status: "pending"` — nothing new, check again next interval

Poll triggers can use the KV store to track state (e.g., "last seen row ID").

### Webhook

The orchestrator calls the trigger component when an HTTP request arrives. The component can:

- Validate the request (check signatures, auth headers)
- Transform the payload
- Return `status: "completed"` with data to start the workflow
- Return `status: "pending"` to reject the request

## Manifest Definition

Trigger type and configuration are declared in the component manifest:

```json
{
  "triggers": {
    "row-added": {
      "description": "Fires when a new row is added",
      "schema": { ... }
    }
  }
}
```

## Host Capabilities

Triggers have access to the same host capabilities as tasks: KV store, config, HTTP, and logging. This allows polling triggers to persist state across invocations.
