# Overview

Fuchsia is composed of five crates that fit together as follows:

```text
┌──────────────────────────────────────────────────────────────┐
│                       Host application                       │
│                                                              │
│  - builds an ActorRegistry (own + Wasm + Lua actors)         │
│  - loads or composes a Graph                                 │
│  - calls Orchestrator::start(&graph)                         │
│  - pushes messages, joins, observes via tracing              │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         │ uses
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ fuchsia-runtime: Graph · ActorRegistry · Orchestrator        │
│                                                              │
│   spawns one tokio task per node,                            │
│   wires bounded mpsc channels per edge                       │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         │ delivers messages to
                         ▼
┌──────────────────────────────────────────────────────────────┐
│ Actors (everything below implements fuchsia_actor::Actor)    │
│                                                              │
│  ┌─────────────┐  ┌──────────────────┐  ┌─────────────────┐  │
│  │ native Rust │  │ fuchsia-actor-wasm│  │ fuchsia-actor-lua│  │
│  │ (your code) │  │  (WasmActor<H>)  │  │   (LuaActor<H>) │  │
│  └─────────────┘  └────────┬─────────┘  └────────┬────────┘  │
│                            │                     │           │
│                  ┌─────────▼──┐         ┌────────▼─────────┐ │
│                  │ WasmHost   │         │  LuaHost         │ │
│                  │ (default   │         │  (default impl   │ │
│                  │  impl OR   │         │   OR host-       │ │
│                  │  host-     │         │   specific)      │ │
│                  │  specific) │         │                  │ │
│                  └────────────┘         └──────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## The mental model

A **Graph** is a JSON-describable directed graph of nodes. Every node names
an actor (by string key) and provides per-node configuration. Edges declare
how outputs flow from one node to the next.

An **Actor** is something that implements
[`fuchsia_actor::Actor`](https://docs.rs/fuchsia-actor) — a trait with one
method, `run(inbox, emit, ctx)`. The actor receives messages on the inbox,
does work, and emits results to the emitter. It does *not* know who
receives its output; that's the orchestrator's job.

The **Orchestrator** is the thing that turns a graph into a running
workflow. It creates one tokio mpsc channel per node, builds an `Emitter`
for each actor by cloning the senders of its downstream nodes, spawns one
tokio task per node, and steps aside. After setup, messages flow through
the channels without any central scheduler involved.

A **WorkflowHandle** is what the host gets back. It exposes `send(value)`
for pushing messages into the entry node, `cancel()` for shutdown, and
`join()` for awaiting completion.

## Why dataflow, not classic actors

Classic actor models (Hewitt, Erlang, Akka) have actors address each other
explicitly: `ctx.send(other_actor_pid, message)`. The communication
topology is encoded in the actor's code.

Fuchsia takes the opposite stance: the topology is configuration. Actors
return values; the graph wires them. This means:

- Workflow authors edit JSON, not Rust/Wasm/Lua.
- Actors stay decoupled from any particular use case.
- Routing changes don't require rebuilding actors.

You give up the ability to do dynamic routing from inside an actor — but
you gain a clean separation between "what this code does" and "where its
output goes." For a workflow engine that's the right trade.

## Three actor flavors, one trait

The same `Actor` trait covers:

1. **Native Rust actors.** Implement the trait directly. Best for
   performance-critical or trusted code, and for protocol adapters (BLE,
   MQTT, Modbus) where you want full control.
2. **Wasm component actors.** Use `WasmActor<H: WasmHost>` from
   `fuchsia-actor-wasm`. Best for third-party plugins or for safe
   sandboxed extension: components are isolated, capability-gated, and
   portable.
3. **Lua script actors.** Use `LuaActor<H: LuaHost>` from
   `fuchsia-actor-lua`. Best for quick scripting, configuration-driven
   transforms, or anywhere a full Wasm build is heavier than the task
   warrants.

The orchestrator doesn't distinguish them — all three are just `Actor`
implementations registered into the `ActorRegistry`.

## Host responsibilities

Fuchsia is deliberately minimal. The host owns:

- **Where actors come from.** A plugin store, a hard-coded list, a
  manifest file — Fuchsia doesn't define this.
- **What capabilities exist.** Fuchsia ships HTTP and a log-routing
  convention. Anything else (KV, MQTT, BLE, custom databases) is defined
  and implemented by the host.
- **Integrity, versioning, install.** When the host loads a Wasm
  component, it verifies digests, picks the version, manages allow-lists.
  Fuchsia just executes what it's handed.
- **Observability.** Fuchsia emits `tracing` events. The host installs
  the subscriber (stdout, OTLP, journald, anything).

This is what keeps the runtime small and the ecosystem flexible.

## Next

- [Runtime](./engine.md) — Graph, Registry, Orchestrator, channels, lifecycle
- [Capabilities](./host-capabilities.md) — HTTP + log routing + injection
- [Host Extensibility](./host-extensibility.md) — adding your own capability layer
