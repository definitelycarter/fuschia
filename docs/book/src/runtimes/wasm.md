# WebAssembly Actors

`fuchsia-actor-wasm` hosts WebAssembly components as Fuchsia actors. Each
`WasmActor` wraps a compiled wasm component; per inbound message it
creates a fresh `wasmtime::Store`, instantiates the component into it,
calls the component's exported `task.execute(ctx, data)`, and emits the
result.

## Anatomy

```rust
pub struct WasmActor<H: WasmHost> {
    engine: Engine,                    // shared, cheap to clone (Arc)
    component: Component,              // compiled, cheap to clone (Arc)
    linker: Arc<Linker<H::State>>,     // built once at build() time
    host: Arc<H>,                      // shared
    epoch_deadline: u64,
}
```

`WasmActor: Clone` is cheap — every field is either `Arc`-backed
internally or wrapped in `Arc`. That matters because the
`ActorRegistry` factory closure clones the actor for each node.

## Builder

```rust
use fuchsia_actor_wasm::{WasmActor, DefaultHost};
use fuchsia_capabilities::http::{ReqwestHttp, AllowedHosts};

let engine = build_wasmtime_engine();  // shared across all actors
let http = Arc::new(ReqwestHttp::new(AllowedHosts::new(["api.example.com"])));
let host = DefaultHost::new(http);

let actor = WasmActor::builder(engine, host)
    .component_from_path("plugins/temp-mapper.wasm")
    .epoch_deadline(1_000_000)
    .build()?;
```

The builder takes three things:

- An `Engine` (one per process is typical; `Engine::clone` is an `Arc`
  bump)
- A `WasmHost` — `DefaultHost` for the canonical world, or a custom host
  for richer capabilities (see [Host Extensibility](../architecture/host-extensibility.md))
- A component source — `component(Component)`, `component_from_path(path)`,
  or `component_from_bytes(Vec<u8>)`

Build the engine with:

```rust
let mut config = wasmtime::Config::new();
config.async_support(true);
config.wasm_component_model(true);
let engine = wasmtime::Engine::new(&config)?;
```

`build()` does the expensive setup *once* — compiles the component if
needed, then constructs the `Linker` against the host's
`add_to_linker`. Per-message dispatch reuses this `Linker` for every
instantiation, so the wiring cost is paid up-front, not per call.

## The contract

Components built against the canonical `actor-component` world export the
`fuchsia:task/task` interface:

```wit
interface task {
  record context {
    execution-id: string,
    node-id: string,
    task-id: string,
  }

  record output {
    data: string,
  }

  execute: func(ctx: context, data: string) -> result<output, string>;
}
```

Inputs and outputs are JSON-serialized strings — the actor handles the
marshalling. Components built with `wit-bindgen` (e.g., via
`cargo-component`) end up writing something like:

```rust
impl exports::fuchsia::task::task::Guest for MyComponent {
  fn execute(
    ctx: exports::fuchsia::task::task::Context,
    data: String,
  ) -> Result<exports::fuchsia::task::task::Output, String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!("executing in node {}", ctx.node_id),
    );

    // ... do work on `data` (parse JSON, transform, etc.) ...

    Ok(exports::fuchsia::task::task::Output {
      data: serde_json::to_string(&result).unwrap(),
    })
  }
}
```

The `actor-component` world also imports `fuchsia:log/log` and
`fuchsia:http/outbound`, both of which `DefaultHost` provides.

## Per-message lifecycle

For each message that arrives on the actor's inbox:

1. `Store<H::State>` is created fresh via `host.fresh_state()`.
2. The component is instantiated into the store via the pre-built
   `Linker`. Wasm memory is brand new — no state leaks between messages.
3. The actor serializes the inbox `Value` to JSON.
4. `task.execute` is invoked. The component runs.
5. The returned string is parsed back to JSON and emitted to downstream.

If anything fails — instantiation, the component traps, the component
returns an `Err(...)` from `execute`, output JSON is invalid — the
actor's `run` loop returns `ActorError::Other`, the spawned task ends,
and `WorkflowHandle::join()` will surface it in that actor's slot.

## What `DefaultHost` gives you

The off-the-shelf `DefaultHost` targets the canonical
`fuchsia:platform/actor-component` world. It satisfies:

- `fuchsia:log/log` — routes calls to `tracing` under target
  `"wasm.component"`. The actor's tokio task is already wrapped in a span
  carrying the `node` id, so log events automatically pick up context.
- `fuchsia:http/outbound` — delegates to the injected `HttpClient`.
  Sync-to-async bridging is done via `futures::executor::block_on`
  (a known wart; see the [Capabilities](../architecture/host-capabilities.md)
  page).

For custom capabilities (MQTT, BLE, anything domain-specific), implement
your own `WasmHost` — see [Host Extensibility](../architecture/host-extensibility.md).

## Building test components

The integration test for `fuchsia-actor-wasm` needs a compiled component
on disk. Build it with [`cargo-component`]:

```bash
cd test-components/test-actor-component
cargo component build --release
```

This produces `target/wasm32-wasip1/release/test_actor_component.wasm`,
which the test loads via `component_from_path`.

[`cargo-component`]: https://github.com/bytecodealliance/cargo-component

## Performance notes

- **Component compilation is the expensive step.** Done once at
  `builder.build()` time. Per-message cost is dominated by
  instantiation + wasm execution.
- **Linker is built once** at `build()` and reused across all
  instantiations. (The legacy task runtime rebuilt the linker per call —
  that overhead is gone.)
- **`Engine::clone`** is an `Arc` bump; one engine for the whole process
  is the recommended shape.
- **No component caching across actors.** Each `WasmActor` instance
  holds its own `Component`. If you want shared compilation across many
  registered actor names, compile once and pass the `Component` directly
  into multiple builders.
