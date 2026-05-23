use crate::host::LuaHost;
use async_trait::async_trait;
use fuchsia_actor::{Actor, ActorError, Context, Emitter, Inbox};
use serde_json::Value;
use std::sync::Arc;

/// A [`fuchsia_actor::Actor`] backed by a Lua script.
///
/// `LuaActor` is generic over a [`LuaHost`] that supplies the host globals
/// the script will call into. Fuchsia ships [`DefaultLuaHost`](crate::DefaultLuaHost)
/// for the canonical capability set; hosts with custom capabilities define
/// their own.
///
/// Per message: a fresh `mlua::Lua` is created, the host's globals are
/// registered, the script source is loaded (defining a global `execute`
/// function), and `execute(ctx, data)` is invoked with the inbound JSON
/// payload. The returned JSON string is emitted to downstream nodes.
///
/// Cheap to clone — `source` and `host` are `Arc`-shared.
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

  fn invoke(&self, input: Value, ctx: &Context) -> Result<Value, ActorError> {
    let lua = mlua::Lua::new();

    self
      .host
      .populate(&lua)
      .map_err(|e| ActorError::Other(format!("lua populate: {e}")))?;

    lua
      .load(self.source.as_str())
      .exec()
      .map_err(|e| ActorError::Other(format!("lua load: {e}")))?;

    let execute: mlua::Function = lua.globals().get("execute").map_err(|_| {
      ActorError::Other("script must define an `execute(ctx, data)` function".into())
    })?;

    let lua_ctx = lua
      .create_table()
      .map_err(|e| ActorError::Other(format!("lua ctx table: {e}")))?;
    lua_ctx
      .set("node_id", ctx.node_id.clone())
      .map_err(|e| ActorError::Other(format!("lua ctx set: {e}")))?;
    lua_ctx
      .set("execution_id", "")
      .map_err(|e| ActorError::Other(format!("lua ctx set: {e}")))?;
    lua_ctx
      .set("task_id", "")
      .map_err(|e| ActorError::Other(format!("lua ctx set: {e}")))?;

    let data = serde_json::to_string(&input)
      .map_err(|e| ActorError::Other(format!("input serialize: {e}")))?;

    let result: String = execute
      .call((lua_ctx, data))
      .map_err(|e| ActorError::Other(format!("lua execute: {e}")))?;

    serde_json::from_str(&result).map_err(|e| ActorError::Other(format!("output parse: {e}")))
  }
}

#[async_trait]
impl<H: LuaHost> Actor for LuaActor<H> {
  async fn run(&self, mut inbox: Inbox, emit: Emitter, ctx: Context) -> Result<(), ActorError> {
    loop {
      tokio::select! {
          _ = ctx.cancelled() => return Ok(()),
          msg = inbox.recv() => match msg {
              Some(v) => {
                  let output = self.invoke(v, &ctx)?;
                  emit.send(output).await?;
              }
              None => return Ok(()),
          }
      }
    }
  }
}
