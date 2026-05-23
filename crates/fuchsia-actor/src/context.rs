use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Context {
  pub node_id: String,
  cancel: CancellationToken,
}

impl Context {
  pub fn new(node_id: impl Into<String>, cancel: CancellationToken) -> Self {
    Self {
      node_id: node_id.into(),
      cancel,
    }
  }

  pub async fn cancelled(&self) {
    self.cancel.cancelled().await
  }

  pub fn is_cancelled(&self) -> bool {
    self.cancel.is_cancelled()
  }
}
