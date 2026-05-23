use crate::host::LuaHost;
use async_trait::async_trait;
use fuchsia_actor::{Actor, ActorError, Context, Emitter, Inbox};
use std::sync::Arc;

/// A [`fuchsia_actor::Actor`] backed by a Lua script.
///
/// `LuaActor` is generic over a [`LuaHost`] that supplies the host globals
/// the script will call into. Fuchsia ships [`DefaultLuaHost`](crate::DefaultLuaHost)
/// for the canonical capability set; hosts with custom capabilities define
/// their own.
///
/// Per actor instance: one `mlua::Lua` is created at the top of `run`, the
/// host's globals are registered (including the `emit` closure that forwards
/// to the actor's outbound channel), and the script source is loaded once.
/// The runtime then drives the script's lifecycle: optional `setup()` →
/// loop(`handle(ctx, message)` per inbox delivery) → optional `teardown()`
/// on cancellation.
///
/// `handle` is required; `setup` and `teardown` are optional globals — the
/// runtime skips them if undefined.
///
/// Cheap to clone — `source` and `host` are `Arc`-shared. Each clone runs
/// its own Lua VM when started.
pub struct LuaActor<H: LuaHost> {
  pub(crate) source: Arc<String>,
  pub(crate) host: Arc<H>,
}

impl<H: LuaHost> Clone for LuaActor<H> {
  fn clone(&self) -> Self {
    Self {
      source: Arc::clone(&self.source),
      host: Arc::clone(&self.host),
    }
  }
}

impl<H: LuaHost> LuaActor<H> {
  /// Start building a `LuaActor` with the given host.
  pub fn builder(host: H) -> crate::LuaActorBuilder<H> {
    crate::LuaActorBuilder::new(host)
  }
}

fn build_ctx(lua: &mlua::Lua, ctx: &Context) -> Result<mlua::Table, ActorError> {
  let lua_ctx = lua
    .create_table()
    .map_err(|e| ActorError::Other(format!("lua ctx table: {e}")))?;
  lua_ctx
    .set("node_id", ctx.node_id.as_str())
    .map_err(|e| ActorError::Other(format!("lua ctx set: {e}")))?;
  lua_ctx
    .set("execution_id", "")
    .map_err(|e| ActorError::Other(format!("lua ctx set: {e}")))?;
  lua_ctx
    .set("task_id", "")
    .map_err(|e| ActorError::Other(format!("lua ctx set: {e}")))?;
  Ok(lua_ctx)
}

#[async_trait]
impl<H: LuaHost> Actor for LuaActor<H> {
  async fn run(&self, mut inbox: Inbox, emit: Emitter, ctx: Context) -> Result<(), ActorError> {
    let lua = mlua::Lua::new();

    self
      .host
      .populate(&lua, emit)
      .map_err(|e| ActorError::Other(format!("lua populate: {e}")))?;

    lua
      .load(self.source.as_str())
      .exec()
      .map_err(|e| ActorError::Other(format!("lua load: {e}")))?;

    let setup_fn: Option<mlua::Function> = lua.globals().get("setup").ok();
    let teardown_fn: Option<mlua::Function> = lua.globals().get("teardown").ok();
    let handle_fn: mlua::Function = lua.globals().get("handle").map_err(|_| {
      ActorError::Other("script must define a `handle(ctx, message)` function".into())
    })?;

    if let Some(setup) = setup_fn {
      let setup_ctx = build_ctx(&lua, &ctx)?;
      setup
        .call::<()>(setup_ctx)
        .map_err(|e| ActorError::Other(format!("lua setup: {e}")))?;
    }

    let loop_result: Result<(), ActorError> = loop {
      let msg = tokio::select! {
        _ = ctx.cancelled() => break Ok(()),
        msg = inbox.recv() => msg,
      };

      let Some(value) = msg else {
        break Ok(());
      };

      let data = match serde_json::to_string(&value) {
        Ok(s) => s,
        Err(e) => break Err(ActorError::Other(format!("input serialize: {e}"))),
      };

      let handle_ctx = match build_ctx(&lua, &ctx) {
        Ok(c) => c,
        Err(e) => break Err(e),
      };

      if let Err(e) = handle_fn.call::<()>((handle_ctx, data)) {
        break Err(ActorError::Other(format!("lua handle: {e}")));
      }
    };

    if let Some(teardown) = teardown_fn {
      match build_ctx(&lua, &ctx) {
        Ok(teardown_ctx) => {
          if let Err(e) = teardown.call::<()>(teardown_ctx) {
            tracing::warn!(error = %e, "lua teardown error");
          }
        }
        Err(e) => tracing::warn!(error = %e, "lua teardown ctx build failed"),
      }
    }

    loop_result
  }
}
