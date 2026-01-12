//! Integration tests for fuscia-trigger-host using a real wasm component.

use std::path::Path;

use fuscia_host::{EngineConfig, KvStore, create_engine};
use fuscia_trigger_host::{
  Event, Status, TriggerHostState, WebhookRequest, execute_trigger, load_component,
};

const TEST_COMPONENT_PATH: &str = "../../test-components/test-trigger-component/target/wasm32-wasip1/release/test_trigger_component.wasm";

#[tokio::test]
async fn test_trigger_poll_pending() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  assert!(
    component_path.exists(),
    "Test component not found at {:?}. Run `cargo component build --release` in test-components/test-trigger-component first.",
    component_path
  );

  let component = load_component(&engine, &component_path).expect("failed to load component");

  // First poll should return pending (no "ready" flag set)
  let state = TriggerHostState::new("exec-123".to_string(), "trigger-456".to_string());
  let result = execute_trigger(&engine, &component, state, Event::Poll, None)
    .await
    .expect("failed to execute trigger");

  assert!(matches!(result.status, Status::Pending));
}

#[tokio::test]
async fn test_trigger_poll_completed() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  if !component_path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &component_path).expect("failed to load component");

  // Create state and pre-set the "ready" flag
  let mut state = TriggerHostState::new("exec-123".to_string(), "trigger-456".to_string());

  // Set the ready flag in KV before execution
  futures::executor::block_on(state.inner.kv.set("ready", "true".to_string()));

  let result = execute_trigger(&engine, &component, state, Event::Poll, None)
    .await
    .expect("failed to execute trigger");

  match result.status {
    Status::Completed(payload) => {
      let output: serde_json::Value =
        serde_json::from_str(&payload).expect("payload should be valid JSON");
      assert_eq!(output["source"], "poll");
    }
    Status::Pending => panic!("expected Completed, got Pending"),
  }
}

#[tokio::test]
async fn test_trigger_webhook() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  if !component_path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &component_path).expect("failed to load component");

  let state = TriggerHostState::new("exec-123".to_string(), "trigger-456".to_string());

  let webhook_request = WebhookRequest {
    method: "POST".to_string(),
    path: "/webhook/test".to_string(),
    headers: vec![("Content-Type".to_string(), "application/json".to_string())],
    body: Some(r#"{"event": "test"}"#.to_string()),
  };

  let result = execute_trigger(
    &engine,
    &component,
    state,
    Event::Webhook(webhook_request),
    None,
  )
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
async fn test_multiple_trigger_executions_isolated() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  if !component_path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &component_path).expect("failed to load component");

  // First execution - set ready flag, should complete
  let mut state1 = TriggerHostState::new("exec-1".to_string(), "trigger-1".to_string());
  futures::executor::block_on(state1.inner.kv.set("ready", "true".to_string()));

  let result1 = execute_trigger(&engine, &component, state1, Event::Poll, None)
    .await
    .expect("first execution failed");

  // Second execution - fresh state, no ready flag, should be pending
  let state2 = TriggerHostState::new("exec-2".to_string(), "trigger-2".to_string());

  let result2 = execute_trigger(&engine, &component, state2, Event::Poll, None)
    .await
    .expect("second execution failed");

  assert!(matches!(result1.status, Status::Completed(_)));
  assert!(matches!(result2.status, Status::Pending));
}
