# Lua Actors

`fuchsia-actor-lua` hosts Lua scripts as Fuchsia actors. Each `LuaActor`
holds a Lua source string and a `LuaHost`; per inbound message it creates
a fresh `mlua::Lua` VM, calls `host.populate(&lua)` to register globals,
loads the script, and invokes a global `execute(ctx, data)` function.

The shape mirrors [`fuchsia-actor-wasm`](./wasm.md) — same trait surface,
same builder pattern, same per-message contract — without the WIT and
bindgen layers.

## Anatomy

```rust
pub struct LuaActor<H: LuaHost> {
    source: Arc<String>,
    host: Arc<H>,
}
```

`LuaActor: Clone` is cheap (two `Arc`s).

## Builder

```rust
use fuchsia_actor_lua::{DefaultLuaHost, LuaActor};
use fuchsia_capabilities::http::{AllowedHosts, ReqwestHttp};

let http = Arc::new(ReqwestHttp::new(AllowedHosts::new(["api.example.com"])));
let host = DefaultLuaHost::new(http);

let actor = LuaActor::builder(host)
    .source(SCRIPT)              // inline string
    // .source_from_path("scripts/transform.lua")  // or from disk
    .build()?;
```

There's no `Engine` to share (Lua VMs are per-message). The host carries
whatever capability handles the script will access.

## The contract

A Lua actor script must define a global `execute` function:

```lua
function execute(ctx, data)
  -- ctx is a table with fields: node_id, execution_id, task_id
  -- data is a JSON-serialized string (the inbound message)

  log.log("info", "node " .. ctx.node_id .. " received " .. data)

  -- ... do work ...

  return '{"processed": true, "node": "' .. ctx.node_id .. '"}'
end
```

Inputs and outputs are JSON strings, same as the Wasm side. The actor
serializes the inbox `Value` before calling, and parses the returned
string back to JSON before emitting.

## Per-message lifecycle

For each message that arrives on the actor's inbox:

1. A fresh `mlua::Lua` is created (`Lua::new()` — cheap; this is the
   intended pattern with the `send` feature enabled).
2. `host.populate(&lua)` registers globals (typically `log` and `http`
   for the default host; custom hosts add more).
3. The script source is loaded and executed, defining the `execute`
   function as a global.
4. A Lua `ctx` table is constructed with the actor's per-message context.
5. `execute(ctx, data)` is called with the JSON-serialized payload.
6. The returned string is parsed back to JSON and emitted.

Lua VM state never persists across messages — same isolation property
as the Wasm side.

## What `DefaultLuaHost` registers

The off-the-shelf `DefaultLuaHost` exposes two globals:

```lua
log.log(level, message)              -- routes to tracing
                                     -- level: "trace" | "debug" | "info" | "warn" | "error"

local resp = http.send({
  method = "POST",
  url = "https://api.example.com/items",
  headers = { ["Content-Type"] = "application/json" },
  body = '{"foo": "bar"}',
})
-- resp.status, resp.headers, resp.body
```

`http.send` returns a Lua table on success; on failure it raises a Lua
runtime error with the `HttpError` display string. Wrap calls in
`pcall` if you want to handle failures inside the script.

## Adding capabilities

The `LuaHost` trait is one method — `populate(&Lua)`. To add capabilities,
write your own host that registers additional globals. See
[Host Extensibility](../architecture/host-extensibility.md) for a worked
example (registering an `mqtt` global).

## Performance notes

- **Per-message Lua creation** is the dominant cost. `mlua::Lua::new`
  with `lua54+vendored` is fast (microseconds) but not free.
- **No script compilation caching.** The script source is re-loaded
  every message. This could be optimized (cache `Function` chunks per
  actor) if profiling shows it matters; for now it matches the simplest
  possible implementation.
- **No persistent Lua state across messages**, by design. If a script
  wants to remember things, the data must round-trip through the inbox
  message (or through an injected capability like KV).

## Test

The lua integration test uses an inline script:

```rust
const SCRIPT: &str = r#"
function execute(ctx, data)
  log.log("info", "lua: node " .. ctx.node_id .. " received " .. data)
  return string.format('{"echoed": %s, "node": "%s"}', data, ctx.node_id)
end
"#;
```

End-to-end run through `fuchsia-runtime`:
`crates/fuchsia-actor-lua/tests/lua_actor.rs`.
