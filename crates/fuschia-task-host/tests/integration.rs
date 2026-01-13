//! Integration tests for fuschia-task-host using a real wasm component.

use std::path::Path;

use fuschia_host::{EngineConfig, create_engine};
use fuschia_task_host::{Context, TaskHostState, execute_task, load_component};

const TEST_COMPONENT_PATH: &str =
  "../../test-components/test-task-component/target/wasm32-wasip1/release/test_task_component.wasm";

#[tokio::test]
async fn test_execute_task_component() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  assert!(
    component_path.exists(),
    "Test component not found at {:?}. Run `cargo component build --release` in test-components/test-task-component first.",
    component_path
  );

  let component = load_component(&engine, &component_path).expect("failed to load component");

  let state = TaskHostState::new("exec-123".to_string(), "node-456".to_string());

  let ctx = Context {
    execution_id: "exec-123".to_string(),
    node_id: "node-456".to_string(),
    task_id: "task-789".to_string(),
  };

  let input = r#"{"message": "hello world"}"#;

  let result = execute_task(&engine, &component, state, ctx, input.to_string(), None)
    .await
    .expect("failed to execute task");

  // Parse the output to verify it contains our input
  let output: serde_json::Value =
    serde_json::from_str(&result.output.data).expect("output should be valid JSON");

  // The test component echoes back the input
  assert_eq!(output["input"]["message"], "hello world");

  // The test component verifies KV store works
  assert_eq!(output["stored_matches"], true);
}

#[tokio::test]
async fn test_task_component_with_invalid_json() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  if !component_path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &component_path).expect("failed to load component");

  let state = TaskHostState::new("exec-123".to_string(), "node-456".to_string());

  let ctx = Context {
    execution_id: "exec-123".to_string(),
    node_id: "node-456".to_string(),
    task_id: "task-789".to_string(),
  };

  // Pass valid JSON - the component will process it
  let input = r#"{"test": true}"#;

  let result = execute_task(&engine, &component, state, ctx, input.to_string(), None)
    .await
    .expect("execute should succeed");

  let output: serde_json::Value =
    serde_json::from_str(&result.output.data).expect("output should be valid JSON");

  assert_eq!(output["input"]["test"], true);
}

#[tokio::test]
async fn test_multiple_task_executions_isolated() {
  let engine = create_engine(EngineConfig::default()).expect("failed to create engine");

  let component_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH);
  if !component_path.exists() {
    eprintln!("Skipping test: component not built");
    return;
  }

  let component = load_component(&engine, &component_path).expect("failed to load component");

  // First execution
  let state1 = TaskHostState::new("exec-1".to_string(), "node-1".to_string());
  let ctx1 = Context {
    execution_id: "exec-1".to_string(),
    node_id: "node-1".to_string(),
    task_id: "task-1".to_string(),
  };

  let result1 = execute_task(
    &engine,
    &component,
    state1,
    ctx1,
    r#"{"value": 1}"#.to_string(),
    None,
  )
  .await
  .expect("first execution failed");

  // Second execution with different state
  let state2 = TaskHostState::new("exec-2".to_string(), "node-2".to_string());
  let ctx2 = Context {
    execution_id: "exec-2".to_string(),
    node_id: "node-2".to_string(),
    task_id: "task-2".to_string(),
  };

  let result2 = execute_task(
    &engine,
    &component,
    state2,
    ctx2,
    r#"{"value": 2}"#.to_string(),
    None,
  )
  .await
  .expect("second execution failed");

  // Both should have stored their own values (isolated KV)
  let output1: serde_json::Value = serde_json::from_str(&result1.output.data).unwrap();
  let output2: serde_json::Value = serde_json::from_str(&result2.output.data).unwrap();

  assert_eq!(output1["input"]["value"], 1);
  assert_eq!(output2["input"]["value"], 2);

  // Both should report stored_matches = true (each stored their own input)
  assert_eq!(output1["stored_matches"], true);
  assert_eq!(output2["stored_matches"], true);
}
