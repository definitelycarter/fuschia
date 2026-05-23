use crate::channel::{Emitter, Inbox};
use crate::context::Context;
use crate::error::ActorError;
use async_trait::async_trait;

#[async_trait]
pub trait Actor: Send + Sync {
  async fn run(&self, inbox: Inbox, emit: Emitter, ctx: Context) -> Result<(), ActorError>;
}
