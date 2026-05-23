# Graph Configuration

A workflow is a `Graph` — a directed graph of nodes with entry-point
semantics. `Graph`, `Node`, and `Edge` are plain Rust types with
`serde::Serialize` / `Deserialize` derives, so workflows can be built
programmatically *or* deserialized from JSON.

## Shape

```rust
pub struct Graph {
    pub entry: String,        // id of the node that receives inbound messages
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

pub struct Node {
    pub id: String,
    pub actor: String,                  // registry key
    pub config: serde_json::Value,      // passed to the actor's factory
}

pub struct Edge {
    pub from: String,
    pub to: String,
}
```

## JSON form

```json
{
  "entry": "ingest",
  "nodes": [
    {
      "id": "ingest",
      "actor": "fuchsia.json-path",
      "config": { "path": "$.temperature" }
    },
    {
      "id": "debounce",
      "actor": "fuchsia.debounce",
      "config": { "window_ms": 500 }
    },
    {
      "id": "store",
      "actor": "iot.local-store.write",
      "config": { "table": "readings" }
    },
    {
      "id": "history",
      "actor": "iot.history.push",
      "config": { "endpoint": "https://history.example.com/readings" }
    }
  ],
  "edges": [
    { "from": "ingest",   "to": "debounce" },
    { "from": "debounce", "to": "store"   },
    { "from": "debounce", "to": "history" }
  ]
}
```

Loaded with `serde_json::from_str::<Graph>(json)` or
`serde_json::from_reader(file)`.

## Semantics

- **`entry`** is the node that receives messages from `WorkflowHandle::send`.
  It must reference an existing node in `nodes`.
- **`nodes[].id`** must be unique within the graph. The orchestrator
  rejects graphs with duplicates.
- **`nodes[].actor`** must resolve in the `ActorRegistry` provided to
  the orchestrator. Unknown keys cause `Orchestrator::start` to fail
  fast.
- **`nodes[].config`** is a free-form JSON value. It's deserialized into
  the type expected by the factory closure registered for that actor.
  The closure decides what the schema is.
- **`edges[].from`** and **`edges[].to`** must reference existing node
  ids. The orchestrator validates this at startup.
- **Edges with the same `from`** become fan-out — the upstream's output
  is delivered to every downstream.
- **Edges with the same `to`** become merge — the downstream's inbox
  interleaves messages from all upstream emitters.

## Common shapes

**Linear chain.**

```json
{
  "entry": "a",
  "nodes": [{"id":"a","actor":"x","config":null},
            {"id":"b","actor":"y","config":null},
            {"id":"c","actor":"z","config":null}],
  "edges": [{"from":"a","to":"b"},{"from":"b","to":"c"}]
}
```

**Fan-out: write to two sinks.**

```json
{
  "entry": "in",
  "nodes": [{"id":"in","actor":"src","config":null},
            {"id":"db","actor":"db.write","config":null},
            {"id":"api","actor":"http.push","config":null}],
  "edges": [{"from":"in","to":"db"},
            {"from":"in","to":"api"}]
}
```

**Merge: two upstreams into one sink.**

```json
{
  "entry": "in",
  "nodes": [{"id":"in","actor":"src","config":null},
            {"id":"left","actor":"transform-a","config":null},
            {"id":"right","actor":"transform-b","config":null},
            {"id":"out","actor":"sink","config":null}],
  "edges": [{"from":"in","to":"left"},
            {"from":"in","to":"right"},
            {"from":"left","to":"out"},
            {"from":"right","to":"out"}]
}
```

This is *merge*, not *join* — `out` will see two messages per input,
interleaved. For "wait for one value from each upstream and combine,"
that's an explicit join-actor's job.

## What graphs don't do

- **No cycles.** Adding back-edges produces a topology the orchestrator
  doesn't currently validate against. There's nothing structurally
  stopping you (channels are just channels), but the behavior under
  feedback loops isn't specified — avoid until intentional cycle support
  lands.
- **No dynamic edges.** Routing is baked at `start()` time. If an actor
  needs conditional output, it's the actor's job to decide what to emit;
  the *set of edges* doesn't change at runtime.
- **No per-edge config.** Edges are pure `from`/`to`. Filtering,
  transforming, or routing on edge content is done by inserting a
  dedicated actor between the two endpoints.

## Loading from JSON in Rust

```rust
use fuchsia_runtime::Graph;
use std::fs;

let graph: Graph = serde_json::from_str(&fs::read_to_string("workflow.json")?)?;
let handle = orchestrator.start(&graph)?;
```

For embedding graphs in Rust code without JSON, construct them directly:

```rust
use fuchsia_runtime::{Edge, Graph, Node};
use serde_json::json;

let graph = Graph {
    entry: "in".into(),
    nodes: vec![
        Node { id: "in".into(),  actor: "src".into(),  config: json!({}) },
        Node { id: "out".into(), actor: "sink".into(), config: json!({}) },
    ],
    edges: vec![Edge { from: "in".into(), to: "out".into() }],
};
```

Either form goes through the same orchestrator path.
