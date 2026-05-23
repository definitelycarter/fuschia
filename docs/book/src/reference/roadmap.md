# Roadmap

Outstanding features, known gaps, and open questions. Rows are removed
when work lands (no strikethrough).

## Features

| Feature | Description | Notes |
|---------|-------------|-------|
| Per-actor retry policy | Configurable retries with backoff applied to a node's `handle` call | `fuchsia-runtime` orchestrator |
| Built-in long-running actors | Debouncer, throttle, window, threshold-over-time as standard `fuchsia.*` actor packs | `fuchsia-runtime` or a separate `fuchsia-builtins` crate |
| Workflow-level allowlist of actor names | Workflows declare which `node.actor` keys they're permitted to invoke; resolver rejects unknown | `fuchsia-runtime::ActorRegistry` |
| Per-instance config plumbing | Forward graph node JSON config to wasm/Lua actors (host import `config.get(key)` or similar). Today actors get no per-instance configuration beyond what the host factory closure bakes in. | `fuchsia-actor-wasm`, `fuchsia-actor-lua`, `fuchsia-runtime` |
| Capability-style device binding | When BLE/MQTT/etc. capabilities land, bind each actor instance to one device handle on the host side so component-side functions never name addresses. | host crates, per-capability WIT |
| Cycle support / persistent topologies | Defined semantics for back-edges within a graph (persistent actor lifecycles are already supported — this is about graph shape). | `fuchsia-runtime` |
| Distributed actors | Documented patterns + sample host code for splitting a graph across processes via MQTT/HTTP transport actors | Likely lives in user/host docs, not core |

## Gaps

### `fuchsia-runtime`

| Gap | Priority |
|-----|----------|
| Channel buffer size is a hardcoded const (`CHANNEL_BUFFER = 32`); not configurable per-node or per-graph | Medium |
| `Orchestrator::start` doesn't validate DAG-ness (no cycle detection) | Medium |
| `WorkflowHandle::join` returns results in spawn order; per-node identification requires the caller to remember the order | Low |

### `fuchsia-actor-wasm`

| Gap | Priority |
|-----|----------|
| No epoch-ticker integration — `epoch_deadline` builder exists but nothing drives the ticker, so timeouts don't fire | Medium |
| Cancellation is checked between `handle` calls, not during; long-running handlers cannot be interrupted mid-flight without epoch interruption | Medium |
| One `Component` per `WasmActor` — shared compilation across actor registrations requires the host to compile once and pass `Component` in | Low (already supported, just undocumented as the recommended path for hot startup) |

### `fuchsia-actor-lua`

| Gap | Priority |
|-----|----------|
| `emit` and `http.send` use `futures::executor::block_on` from sync Lua callbacks; works but unprincipled in a tokio context | Medium |
| Cancellation is checked between `handle` calls, not during; long-running Lua callbacks cannot be interrupted mid-flight | Medium |

### `fuchsia-capabilities`

| Gap | Priority |
|-----|----------|
| `HttpClient` is the only trait; no `KvStore`, no `Secrets`, no `FsScope` (intentional — hosts define these — but worth revisiting if convergence emerges) | N/A |

## Open Questions

| Question | Context |
|----------|---------|
| Should node IDs become `Arc<str>` throughout? | Per-message `node_id.clone()` shows up in both Wasm and Lua actors. Trivial cost individually; could compound. Profiled? Not yet. |
| Schema annotation for actor configs | Today each factory closure dictates its `Cfg` type; no machine-readable schema for tooling. Could be derived via `schemars` if we wanted plugin-store UI. |
| Workflow-level capability declarations | If/when an allowlist on `node.actor` keys exists, should it extend to per-actor capability configs too? |
| Replay / observability for inbound messages | Should the runtime support inspecting in-flight messages on channels for debugging? |

## Housekeeping

| Item | Priority |
|------|----------|
| Examples directory still has pre-actor JSON workflow files; needs new examples that match the actor `Graph` shape | Medium |
| Architecture-page rewrites are a single pass per topic; deep-dive content for each capability still TBD | Low |
