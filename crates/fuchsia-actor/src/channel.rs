use crate::error::ActorError;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum MessageValue {
  Json(serde_json::Value),
  Binary(Vec<u8>),
  Empty,
}

#[derive(Clone, Debug)]
pub struct Message {
  pub type_: String,
  pub correlation_id: Option<String>,
  pub value: MessageValue,
}

pub struct MessageBuilder {
  type_: String,
  correlation_id: Option<String>,
}

impl Message {
  pub fn with_type(type_: impl Into<String>) -> MessageBuilder {
    MessageBuilder {
      type_: type_.into(),
      correlation_id: None,
    }
  }
}

impl MessageBuilder {
  pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
    self.correlation_id = Some(id.into());
    self
  }

  pub fn json(self, value: serde_json::Value) -> Message {
    Message {
      type_: self.type_,
      correlation_id: self.correlation_id,
      value: MessageValue::Json(value),
    }
  }

  pub fn binary(self, bytes: Vec<u8>) -> Message {
    Message {
      type_: self.type_,
      correlation_id: self.correlation_id,
      value: MessageValue::Binary(bytes),
    }
  }

  pub fn empty(self) -> Message {
    Message {
      type_: self.type_,
      correlation_id: self.correlation_id,
      value: MessageValue::Empty,
    }
  }
}

pub struct Inbox {
  rx: mpsc::Receiver<Message>,
}

impl Inbox {
  pub fn new(rx: mpsc::Receiver<Message>) -> Self {
    Self { rx }
  }

  pub async fn recv(&mut self) -> Option<Message> {
    let msg = self.rx.recv().await;
    tracing::trace!(received = msg.is_some(), "inbox.recv");
    msg
  }
}

pub struct Emitter {
  senders: Vec<mpsc::Sender<Message>>,
}

impl Emitter {
  pub fn new(senders: Vec<mpsc::Sender<Message>>) -> Self {
    Self { senders }
  }

  pub async fn send(&self, msg: Message) -> Result<(), ActorError> {
    tracing::trace!(downstream = self.senders.len(), "emitter.send");
    match self.senders.split_last() {
      None => Ok(()),
      Some((last, rest)) => {
        for sender in rest {
          sender
            .send(msg.clone())
            .await
            .map_err(|e| ActorError::Send(e.to_string()))?;
        }
        last
          .send(msg)
          .await
          .map_err(|e| ActorError::Send(e.to_string()))
      }
    }
  }
}
