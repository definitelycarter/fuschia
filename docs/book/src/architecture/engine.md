# Runtime

`fuchsia-runtime` is the engine. It defines three things — `Graph`,
`ActorRegistry`, and `Orchestrator` — plus the `WorkflowHandle` you get
back when you start a workflow.

## Graph

A `Graph` is a directed graph with named nodes and edges:

```rust
pub struct Graph {
    pub entry: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

pub struct Node {
    pub id: String,            // unique within the graph
    pub actor: String,         // key in the ActorRegistry
    pub config: serde_json::Value,  // passed to the actor's factory
}

pub struct Edge {
    pub from: String,
    pub to: String,
}
```

`Graph`, `Node`, and `Edge` all derive `Serialize` and `Deserialize`, so a
workflow is just a JSON document. See [Configuration](../workflows/config.md)
for the on-disk shape and examples.

## ActorRegistry

The registry maps string keys (e.g. `"mqtt.publish"`, `"fuchsia.debounce"`)
to factory closures that produce actor instances:

```rust
let mut registry = ActorRegistry::new();

registry.register::<MyActor, MyConfig, _>("my.actor", |cfg| {
    MyActor::new(cfg.field)
});
```

The closure is called once per node at workflow startup, with the node's
`config` deserialized as the closure's expected type. The actor it returns
is stored as `Arc<dyn Actor>` for the orchestrator to spawn.

This is the **capability injection point**: the closure is registered with
whatever handles the actor needs (HTTP clients, KV stores, MQTT brokers)
already closed-over. By the time the workflow runs, every actor instance
has its dependencies baked in.

## Orchestrator

The orchestrator turns a graph into a running workflow. It's stateless
itself; one orchestrator can start many independent workflows.

```rust
let orchestrator = Orchestrator::new(Arc::new(registry));
let handle = orchestrator.start(&graph)?;
```

What `start` actually does:

1. **Create channels.** For each node in the graph, create one
   `tokio::sync::mpsc` channel. Buffer size is currently a constant
   (32) and is the per-edge backpressure boundary.
2. **Build emitters.** For each node, collect the senders of all its
   downstream nodes into a single `Emitter`. When the node emits, it
   sends to *all* downstream senders in order — that's fan-out.
3. **Instantiate actors.** Resolve each `node.actor` name through the
   registry, deserializing `node.config` into the closure's expected
   type. The result is `Arc<dyn Actor>`.
4. **Spawn tasks.** For each node, `tokio::spawn` a task that calls
   `actor.run(inbox, emit, ctx)`. The task is wrapped in a `tracing` span
   carrying the `node` id and the `kind` (actor name) so all events
   emitted from within the actor inherit that context.
5. **Hand back a handle.** Return a `WorkflowHandle` holding the entry
   node's `Sender`, a shared `CancellationToken`, and a `Vec<JoinHandle>`.

After `start` returns, the orchestrator is **done**. Messages flow through
the channels without any central scheduler. The runtime is just whatever
the actors do plus whatever tokio does to drive them.

## WorkflowHandle

```rust
impl WorkflowHandle {
    pub async fn send(&self, value: Value) -> Result<(), ActorError>;
    pub fn cancel(&self);
    pub async fn join(self) -> Vec<Result<(), ActorError>>;
}
```

- `send` pushes a message into the entry node's inbox. If the channel is
  full, it awaits (backpressure).
- `cancel` triggers the shared `CancellationToken`. Every actor is in a
  `tokio::select!` that includes `ctx.cancelled()`, so they exit cleanly.
- `join` drops the entry sender (triggering a completion cascade — each
  actor's inbox closes when its upstream senders are dropped, the actor
  exits, its emitter is dropped, the next actor's inbox closes, etc.),
  then awaits all spawned tasks and returns one `Result` per actor in
  spawn order.

## Channel topology semantics

Some shapes that come for free from "one mpsc channel per node":

- **Linear chain.** Each node has one upstream sender, one inbox. The
  Emitter has one sender. Messages flow straight through.
- **Fan-out (one → many).** A node's Emitter holds multiple downstream
  senders. On `emit.send(value)`, it sends to each in turn (cloning
  except for the last). Each downstream sees the same value.
- **Fan-in / merge (many → one).** Multiple upstream nodes hold senders
  for the same downstream channel. The downstream's inbox interleaves
  messages as they arrive. This is *merge* semantics — not a synchronous
  join. If you need "wait for one value from each upstream and combine,"
  that's a dedicated actor's job (e.g., a `fuchsia.join` builtin).
- **Backpressure.** Bounded mpsc means a slow downstream blocks a fast
  upstream on `emit.send().await`. No further configuration needed.

## Cancellation and completion

The orchestrator holds a `CancellationToken` and gives each spawned task a
`Context` containing a clone. Every actor's `run` loop is expected to
include a `select!` arm on `ctx.cancelled()`:

```rust
tokio::select! {
    _ = ctx.cancelled() => return Ok(()),
    msg = inbox.recv() => match msg {
        Some(v) => { /* process */ }
        None => return Ok(()),
    }
}
```

There are two ways the workflow can wind down:

- **Cancellation.** `handle.cancel()` triggers the token. All actors exit
  via the `_ = ctx.cancelled()` arm.
- **Completion cascade.** `handle.join()` drops the entry sender. The
  entry actor's inbox closes (`recv` returns `None`), it exits, its
  emitter is dropped, the channels feeding its downstreams close, and
  EOF cascades through the graph. Same shape as Unix pipes.

Either way, `join` returns one `Result<(), ActorError>` per actor in
spawn order. The host can inspect each — usually they're all `Ok(())`;
an `Err(...)` indicates a node's `run` returned an error mid-flight.

## What's not in the orchestrator

Some things you might expect a scheduler to do that this one deliberately
doesn't:

- **No retry logic.** An actor that wants to retry handles it inside its
  `run` loop, with its own backoff state. Different actors have different
  retry needs; the orchestrator stays out of it.
- **No timeouts.** If a node needs a deadline, it uses `tokio::time::timeout`
  inside its `run` loop. Cross-graph timeouts are a host concern.
- **No dynamic routing.** Routing is graph configuration, period. If a
  node's behavior depends on its input, that's the actor's logic — but
  the *set of edges* doesn't change at runtime.
- **No persistence.** Messages are in-memory channels. If you want
  durable delivery, build it into a specific actor (e.g., one that writes
  to a queue before emitting). The orchestrator doesn't checkpoint.

These are intentional. The orchestrator is small so the actors can be
specific.
