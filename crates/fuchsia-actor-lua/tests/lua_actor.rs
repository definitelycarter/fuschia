//! End-to-end integration test: register a `LuaActor<DefaultLuaHost>` with
//! `fuchsia-runtime`, push a JSON payload through, and assert the script
//! echoed it back.

use async_trait::async_trait;
use fuchsia_actor::{Actor, ActorError, Context, Emitter, Inbox};
use fuchsia_actor_lua::{DefaultLuaHost, LuaActor};
use fuchsia_capabilities::http::{AllowedHosts, ReqwestHttp};
use fuchsia_runtime::{ActorRegistry, Edge, Graph, Node, Orchestrator};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct Recorder {
  out: Arc<Mutex<Vec<Value>>>,
}

#[async_trait]
impl Actor for Recorder {
  async fn run(&self, mut inbox: Inbox, _emit: Emitter, ctx: Context) -> Result<(), ActorError> {
    loop {
      tokio::select! {
          _ = ctx.cancelled() => return Ok(()),
          msg = inbox.recv() => match msg {
              Some(v) => self.out.lock().unwrap().push(v),
              None => return Ok(()),
          }
      }
    }
  }
}

const SCRIPT: &str = r#"
function handle(ctx, message)
  log.log("info", "lua: node " .. ctx.node_id .. " received " .. message)
  emit(string.format('{"echoed": %s, "node": "%s"}', message, ctx.node_id))
end
"#;

#[tokio::test]
async fn lua_actor_runs_inline_script_end_to_end() {
  // No outbound HTTP in this test, but DefaultLuaHost needs an HttpClient
  // to wire the http global. Empty allow-list rejects any call.
  let http = Arc::new(ReqwestHttp::new(AllowedHosts::default()));
  let host = DefaultLuaHost::new(http);

  let actor = LuaActor::builder(host)
    .source(SCRIPT)
    .build()
    .expect("build LuaActor");

  let out = Arc::new(Mutex::new(Vec::new()));

  let mut registry = ActorRegistry::new();
  {
    let actor = actor.clone();
    registry.register::<LuaActor<DefaultLuaHost>, Value, _>("test.lua", move |_| actor.clone());
  }
  {
    let out = out.clone();
    registry.register::<Recorder, Value, _>("recorder", move |_| Recorder { out: out.clone() });
  }

  let graph = Graph {
    entry: "lua".into(),
    nodes: vec![
      Node {
        id: "lua".into(),
        actor: "test.lua".into(),
        config: Value::Null,
      },
      Node {
        id: "rec".into(),
        actor: "recorder".into(),
        config: Value::Null,
      },
    ],
    edges: vec![Edge {
      from: "lua".into(),
      to: "rec".into(),
    }],
  };

  let orch = Orchestrator::new(Arc::new(registry));
  let handle = orch.start(&graph).expect("start workflow");

  handle.send(json!(42)).await.expect("send input");

  let results = handle.join().await;
  for (i, r) in results.iter().enumerate() {
    assert!(r.is_ok(), "actor {i} failed: {r:?}");
  }

  let recorded = out.lock().unwrap();
  assert_eq!(recorded.len(), 1, "expected one output, got {recorded:?}");

  let output = &recorded[0];
  assert_eq!(output["echoed"], json!(42));
  assert_eq!(output["node"], json!("lua"));
}
