use crate::error::ActorError;
use serde_json::Value;
use tokio::sync::mpsc;

pub struct Inbox {
  rx: mpsc::Receiver<Value>,
}

impl Inbox {
  pub fn new(rx: mpsc::Receiver<Value>) -> Self {
    Self { rx }
  }

  pub async fn recv(&mut self) -> Option<Value> {
    let msg = self.rx.recv().await;
    tracing::trace!(received = msg.is_some(), "inbox.recv");
    msg
  }
}

pub struct Emitter {
  senders: Vec<mpsc::Sender<Value>>,
}

impl Emitter {
  pub fn new(senders: Vec<mpsc::Sender<Value>>) -> Self {
    Self { senders }
  }

  pub async fn send(&self, value: Value) -> Result<(), ActorError> {
    tracing::trace!(downstream = self.senders.len(), "emitter.send");
    match self.senders.split_last() {
      None => Ok(()),
      Some((last, rest)) => {
        for sender in rest {
          sender
            .send(value.clone())
            .await
            .map_err(|e| ActorError::Send(e.to_string()))?;
        }
        last
          .send(value)
          .await
          .map_err(|e| ActorError::Send(e.to_string()))
      }
    }
  }
}
