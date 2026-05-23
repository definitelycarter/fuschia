# Fuchsia

Fuchsia is an actor-based dataflow runtime. Workflows are graphs of actors
connected by tokio mpsc channels; the graph is declared in JSON, and routing
between nodes lives in the graph (configuration), not in actor code. Each
actor is either a native Rust implementation, a Wasm component, or a Lua
script, all behind a single `Actor` trait.

Hosts compose a runtime by registering actors into an `ActorRegistry` and
starting graphs against an `Orchestrator`. Universal capabilities (HTTP, log)
are provided by Fuchsia; domain-specific capabilities (MQTT, BLE, etc.) are
defined and registered by the host.

## Project Structure

- `crates/`
  - `fuchsia-actor` — `Actor` trait + `Inbox` / `Emitter` / `Context` / `ActorError`.
    The minimal API surface third-party actor packs depend on.
  - `fuchsia-runtime` — The engine. `Graph`, `ActorRegistry` (closure-based
    factory with constructor-injected deps), `Orchestrator` that wires tokio
    mpsc channels per-edge and spawns one task per node. `WorkflowHandle`
    exposes `send` / `cancel` / `join`. Criterion benches under `benches/`.
  - `fuchsia-capabilities` — Universal capability traits. Currently just
    `http::HttpClient` async trait + `AllowedHosts` policy + `ReqwestHttp`
    default impl. Hosts inject these into actor constructors.
  - `fuchsia-actor-wasm` — Wasm-component-hosting `Actor` impl, generic over
    a `WasmHost` trait that owns the world-specific bindgen and host-function
    wiring. Ships `DefaultHost` for the canonical `actor-component` world
    (log → tracing, http → injected `HttpClient`).
  - `fuchsia-actor-lua` — Lua-script-hosting `Actor` impl, generic over a
    `LuaHost` trait that owns Lua-global registration. Ships `DefaultLuaHost`
    for the canonical capability set, matching the Wasm side.
- `wit/` — WIT definitions used by `fuchsia-actor-wasm` and components
  - `world.wit` — `actor-platform` world (log + http imports) and `actor-component`
    world (extends platform, exports task)
  - `deps/fuchsia-task/task.wit` — Task interface: `execute(ctx, data) -> result`
  - `deps/fuchsia-log/log.wit` — Log interface routed to `tracing` host-side
  - `deps/fuchsia-http/outbound.wit` — Outbound HTTP interface backed by
    `fuchsia-capabilities::http::HttpClient`
  - `deps/wasi_*.wit` — WASI dependencies used by built components
- `test-components/test-actor-component/` — Workspace-excluded crate that
  compiles to a wasm component for the `fuchsia-actor-wasm` integration test.
  Requires `cargo component build --release` before running the test.
- `examples/` — Sample workflow JSON files.
- `docs/` — Design documentation (predates the actor refactor; treat as
  historical until rewritten).
- `.claude/skills/` — Per-skill instructions (commit, bench, docs).

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo fmt
```

Benches (criterion): `cargo bench -p fuchsia-runtime --bench <name>` —
see `.claude/skills/bench/SKILL.md` for the targeted before/after workflow.

## Guidelines

- Follow Rust idioms and best practices
- Use `cargo fmt` before committing
- Ensure all tests pass with `cargo test --workspace`
- Add tests for new functionality
- Do not automatically commit or push to this repository — wait for explicit user approval
- Avoid `clone()` in production code — provide justification if proposing it (acceptable in tests; refcount-bumping clones of `Arc` / `mpsc::Sender` / `CancellationToken` / `Engine` / `Component` are accepted with brief justification)
- Avoid `unwrap()`, `expect()`, and other panic-prone error handling in production code (acceptable in tests and bench setup; iter-body bench panics are acceptable as invariant assertions)
- Avoid `.ok()` to silently discard errors in production code — propagate errors with `?` or `map_err` instead (acceptable in tests and in `sort_by` closures where returning `Result` is not possible)
