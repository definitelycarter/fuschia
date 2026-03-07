# JavaScript Runtime

> **Status: Future**

A JavaScript runtime would execute nodes as JS scripts in an embedded V8 or lightweight JS engine.

## Potential Engines

| Engine | Crate | Notes |
|--------|-------|-------|
| **V8** (via Deno) | `deno_core` | Most mature, full ES2024+ support. Deno is built in Rust. |
| **QuickJS** | `rquickjs` | Lightweight, embeddable, no C++ dependency. Limited ES2020. |
| **Boa** | `boa_engine` | Pure Rust JS engine. Less mature but no foreign dependencies. |

## Why JS

- Largest ecosystem of developers
- Rich standard library and npm ecosystem
- Good for complex data transformation and business logic
- Many existing n8n users write JS/TS

## Host Capability Wiring

Capabilities would be exposed as module imports or global objects:

```javascript
// KV store
const value = await kv.get("my_key");
await kv.set("my_key", "my_value");

// Config
const apiKey = config.get("api_key");

// Logging
log("info", "processing item");

// HTTP
const response = await http.request({
    method: "GET",
    url: "https://api.example.com/data",
});
```

Same pattern as Lua — each binding is thin glue to the shared host capability implementation.

## Open Questions

- **Which engine?** Deno's `deno_core` is most capable but adds significant binary size. QuickJS via `rquickjs` is lighter but less compatible. Boa is pure Rust but least mature.
- **TypeScript support?** Would need a transpilation step or a TS-aware engine.
- **npm modules?** Supporting `require()`/`import` of npm packages adds significant complexity.
- **Priority**: Lua and wasmtime come first. JS is a future addition based on demand.
