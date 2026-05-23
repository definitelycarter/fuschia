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
    message: String,
  ) -> Result<(), String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!("test-actor-component: handle node {} message {message}", ctx.node_id),
    );

    let output = format!(r#"{{"echoed": {message}, "node": "{}"}}"#, ctx.node_id);
    fuchsia::actor::emit::send(&output)
  }

  fn teardown(ctx: exports::fuchsia::actor::actor::Context) -> Result<(), String> {
    fuchsia::log::log::log(
      fuchsia::log::log::Level::Info,
      &format!("test-actor-component: teardown node {}", ctx.node_id),
    );
    Ok(())
  }
}
