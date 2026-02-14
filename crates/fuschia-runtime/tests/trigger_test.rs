//! Integration tests for TriggerExecutor using a real wasm component.

use std::path::Path;
use std::sync::Arc;

use fuschia_host::{EngineConfig, create_engine};
use fuschia_runtime::{RuntimeError, TriggerExecutor, TriggerInput};
use fuschia_trigger_host::{Event, Status, WebhookRequest, load_component};
use tokio_util::sync::CancellationToken;

const TEST_COMPONENT_PATH: &str = "../../test-components/test-trigger-component/target/wasm32-wasip1/release/test_trigger_component.wasm";

fn component_path() -> std::path::PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH)
}

#[tokio::test]
async fn test_trigger_poll_pending() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TriggerExecutor::new(engine.clone());

  let path = component_path();
  assert!(
    path.exists(),
    "Test component not found at {:?}. Run `cargo component build --release` in test-components/test-trigger-component first.",
    path
  );

  let component = load_component(&engine, &path).expect("failed to load component");
  let cancel = CancellationToken::new();

  let input = TriggerInput {
    execution_id: "exec-123".to_string(),
    node_id: "trigger-456".to_string(),
    event: Event::Poll,
    timeout_ms: None,
  };

  let result = executor
    .execute(&component, input, cancel)
    .await
    .expect("failed to execute trigger");

  assert!(matches!(result.status, Status::Pending));
}

#[tokio::test]
async fn test_trigger_webhook() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TriggerExecutor::new(engine.clone());

  let path = component_path();
  if !path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &path).expect("failed to load component");
  let cancel = CancellationToken::new();

  let input = TriggerInput {
    execution_id: "exec-123".to_string(),
    node_id: "trigger-456".to_string(),
    event: Event::Webhook(WebhookRequest {
      method: "POST".to_string(),
      path: "/webhook/test".to_string(),
      headers: vec![("Content-Type".to_string(), "application/json".to_string())],
      body: Some(r#"{"event": "test"}"#.to_string()),
    }),
    timeout_ms: None,
  };

  let result = executor
    .execute(&component, input, cancel)
    .await
    .expect("failed to execute trigger");

  match result.status {
    Status::Completed(payload) => {
      let output: serde_json::Value =
        serde_json::from_str(&payload).expect("payload should be valid JSON");
      assert_eq!(output["source"], "webhook");
      assert_eq!(output["method"], "POST");
      assert_eq!(output["path"], "/webhook/test");
    }
    Status::Pending => panic!("expected Completed, got Pending"),
  }
}

#[tokio::test]
async fn test_trigger_cancelled() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TriggerExecutor::new(engine.clone());

  let path = component_path();
  if !path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &path).expect("failed to load component");
  let cancel = CancellationToken::new();
  cancel.cancel();

  let input = TriggerInput {
    execution_id: "exec-123".to_string(),
    node_id: "trigger-456".to_string(),
    event: Event::Poll,
    timeout_ms: None,
  };

  let result = executor.execute(&component, input, cancel).await;

  assert!(matches!(result, Err(RuntimeError::Cancelled)));
}
