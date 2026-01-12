use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Trait for execution-scoped key-value storage.
///
/// Components use this to persist state during execution.
/// The scope is per-execution - different executions cannot see each other's data.
///
/// This trait is async to support networked backends like Redis.
pub trait KvStore: Send + Sync {
  /// Get a value by key.
  fn get(&self, key: &str) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>>;

  /// Set a value.
  fn set(&mut self, key: &str, value: String) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

  /// Delete a value.
  fn delete(&mut self, key: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// In-memory KV store implementation.
///
/// Suitable for single-execution use or testing.
#[derive(Debug, Default)]
pub struct InMemoryKvStore {
  data: HashMap<String, String>,
}

impl InMemoryKvStore {
  pub fn new() -> Self {
    Self::default()
  }
}

impl KvStore for InMemoryKvStore {
  fn get(&self, key: &str) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>> {
    let value = self.data.get(key).cloned();
    Box::pin(async move { value })
  }

  fn set(&mut self, key: &str, value: String) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    self.data.insert(key.to_string(), value);
    Box::pin(async {})
  }

  fn delete(&mut self, key: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    self.data.remove(key);
    Box::pin(async {})
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_in_memory_kv_store() {
    let mut store = InMemoryKvStore::new();

    assert_eq!(store.get("key").await, None);

    store.set("key", "value".to_string()).await;
    assert_eq!(store.get("key").await, Some("value".to_string()));

    store.set("key", "updated".to_string()).await;
    assert_eq!(store.get("key").await, Some("updated".to_string()));

    store.delete("key").await;
    assert_eq!(store.get("key").await, None);
  }
}
