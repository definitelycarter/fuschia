//! Integration tests for TaskExecutor using a real wasm component.

use std::path::Path;
use std::sync::Arc;

use fuschia_host::{EngineConfig, create_engine};
use fuschia_runtime::{RuntimeError, TaskExecutor, TaskInput};
use fuschia_task_host::load_component;
use tokio_util::sync::CancellationToken;

const TEST_COMPONENT_PATH: &str =
  "../../test-components/test-task-component/target/wasm32-wasip1/release/test_task_component.wasm";

fn component_path() -> std::path::PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH)
}

#[tokio::test]
async fn test_execute_component() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TaskExecutor::new(engine.clone());

  let path = component_path();
  assert!(
    path.exists(),
    "Test component not found at {:?}. Run `cargo component build --release` in test-components/test-task-component first.",
    path
  );

  let component = load_component(&engine, &path).expect("failed to load component");
  let cancel = CancellationToken::new();

  let input = TaskInput {
    task_id: "task-789".to_string(),
    execution_id: "exec-123".to_string(),
    node_id: "node-456".to_string(),
    input: serde_json::json!({"message": "hello world"}),
    resolved_input: serde_json::json!({"message": "hello world"}),
    timeout_ms: None,
  };

  let result = executor
    .execute(&component, input, cancel)
    .await
    .expect("failed to execute component");

  assert_eq!(result.node_id, "node-456");
  assert_eq!(result.task_id, "task-789");
  assert_eq!(result.output["input"]["message"], "hello world");
  assert_eq!(result.output["stored_matches"], true);
}

#[tokio::test]
async fn test_execute_component_with_different_input() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TaskExecutor::new(engine.clone());

  let path = component_path();
  if !path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &path).expect("failed to load component");
  let cancel = CancellationToken::new();

  let input = TaskInput {
    task_id: "task-789".to_string(),
    execution_id: "exec-123".to_string(),
    node_id: "node-456".to_string(),
    input: serde_json::json!({"test": true}),
    resolved_input: serde_json::json!({"test": true}),
    timeout_ms: None,
  };

  let result = executor
    .execute(&component, input, cancel)
    .await
    .expect("execute should succeed");

  assert_eq!(result.output["input"]["test"], true);
}

#[tokio::test]
async fn test_multiple_executions_isolated() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TaskExecutor::new(engine.clone());

  let path = component_path();
  if !path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &path).expect("failed to load component");

  let input1 = TaskInput {
    task_id: "task-1".to_string(),
    execution_id: "exec-1".to_string(),
    node_id: "node-1".to_string(),
    input: serde_json::json!({"value": 1}),
    resolved_input: serde_json::json!({"value": 1}),
    timeout_ms: None,
  };

  let result1 = executor
    .execute(&component, input1, CancellationToken::new())
    .await
    .expect("first execution failed");

  let input2 = TaskInput {
    task_id: "task-2".to_string(),
    execution_id: "exec-2".to_string(),
    node_id: "node-2".to_string(),
    input: serde_json::json!({"value": 2}),
    resolved_input: serde_json::json!({"value": 2}),
    timeout_ms: None,
  };

  let result2 = executor
    .execute(&component, input2, CancellationToken::new())
    .await
    .expect("second execution failed");

  assert_eq!(result1.output["input"]["value"], 1);
  assert_eq!(result2.output["input"]["value"], 2);
  assert_eq!(result1.output["stored_matches"], true);
  assert_eq!(result2.output["stored_matches"], true);
}

#[tokio::test]
async fn test_cancelled_execution() {
  let engine = Arc::new(create_engine(EngineConfig::default()).expect("failed to create engine"));
  let executor = TaskExecutor::new(engine.clone());

  let path = component_path();
  if !path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &path).expect("failed to load component");
  let cancel = CancellationToken::new();
  cancel.cancel();

  let input = TaskInput {
    task_id: "task-789".to_string(),
    execution_id: "exec-123".to_string(),
    node_id: "node-456".to_string(),
    input: serde_json::json!({}),
    resolved_input: serde_json::json!({}),
    timeout_ms: None,
  };

  let result = executor.execute(&component, input, cancel).await;

  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), RuntimeError::Cancelled));
}
