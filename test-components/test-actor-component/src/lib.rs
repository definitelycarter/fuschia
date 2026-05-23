wit_bindgen::generate!({
    path: "../../wit",
    world: "actor-component",
    generate_all,
});

struct TestActor;

export!(TestActor);

impl exports::fuchsia::task::task::Guest for TestActor {
  fn execute(
    ctx: exports::fuchsia::task::task::Context,
    data: String,
  ) -> Result<exports::fuchsia::task::task::Output, String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!("test-actor-component: executing node {} with data {data}", ctx.node_id),
    );

    let output = format!(r#"{{"echoed": {data}, "node": "{}"}}"#, ctx.node_id);
    Ok(exports::fuchsia::task::task::Output { data: output })
  }
}
