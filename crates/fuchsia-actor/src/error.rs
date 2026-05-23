use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActorError {
  #[error("unknown actor: {0}")]
  UnknownActor(String),

  #[error("unknown node referenced by graph: {0}")]
  UnknownNode(String),

  #[error("failed to deserialize actor config: {0}")]
  Config(#[from] serde_json::Error),

  #[error("channel send failed: {0}")]
  Send(String),

  #[error("actor task panicked")]
  Panic,

  #[error("{0}")]
  Other(String),
}
