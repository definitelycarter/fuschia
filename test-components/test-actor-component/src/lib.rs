wit_bindgen::generate!({
    path: "../../wit",
    world: "actor-component",
    generate_all,
});

struct TestActor;

export!(TestActor);

impl exports::fuchsia::actor::actor::Guest for TestActor {
  fn setup(ctx: exports::fuchsia::actor::actor::Context) -> Result<(), String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!("test-actor-component: setup node {}", ctx.node_id),
    );
    Ok(())
  }

  fn handle(
    ctx: exports::fuchsia::actor::actor::Context,
    msg: fuchsia::actor::types::Payload,
  ) -> Result<(), String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!(
        "test-actor-component: handle node {} type {}",
        ctx.node_id, msg.type_
      ),
    );

    let echoed_str = String::from_utf8_lossy(&msg.value).into_owned();
    let out_json = format!(
      r#"{{"echoed": {}, "node": "{}"}}"#,
      echoed_str, ctx.node_id
    );

    fuchsia::actor::emit::send(&fuchsia::actor::types::Payload {
      type_: "echo".to_string(),
      correlation_id: msg.correlation_id,
      value: out_json.into_bytes(),
    })
  }

  fn teardown(ctx: exports::fuchsia::actor::actor::Context) -> Result<(), String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!("test-actor-component: teardown node {}", ctx.node_id),
    );
    Ok(())
  }
}
