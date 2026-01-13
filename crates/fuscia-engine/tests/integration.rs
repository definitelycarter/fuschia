//! Integration tests for fuscia-engine using real wasm components.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use fuscia_engine::{EngineConfig, ExecutionError, WorkflowEngine, WorkflowRunner};
use fuscia_workflow::{LockedComponent, LockedTrigger, Node, NodeType, Workflow};
use serde_json::json;
use tokio_util::sync::CancellationToken;

/// Path to the test task component relative to the test crate.
const TEST_COMPONENT_PATH: &str =
  "../../test-components/test-task-component/target/wasm32-wasip1/release/test_task_component.wasm";

fn test_component_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH)
}

fn component_exists() -> bool {
  test_component_path().exists()
}

/// Create an engine config that points to a temp directory where we'll
/// symlink the test component.
fn create_test_engine_config() -> (EngineConfig, tempfile::TempDir) {
  let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

  // Create the component directory structure: <base>/test-task/1.0.0/component.wasm
  let component_dir = temp_dir.path().join("test-task").join("1.0.0");
  std::fs::create_dir_all(&component_dir).expect("failed to create component dir");

  // Symlink or copy the test component
  let src = test_component_path();
  let dst = component_dir.join("component.wasm");

  #[cfg(unix)]
  std::os::unix::fs::symlink(&src, &dst).expect("failed to symlink component");

  #[cfg(windows)]
  std::fs::copy(&src, &dst).expect("failed to copy component");

  let config = EngineConfig {
    component_base_path: temp_dir.path().to_path_buf(),
  };

  (config, temp_dir)
}

/// Create a test LockedComponent with default schema.
fn test_locked_component() -> LockedComponent {
  LockedComponent {
    name: "test-task".to_string(),
    version: "1.0.0".to_string(),
    digest: "sha256:abc123".to_string(),
    task_name: "execute".to_string(),
    input_schema: json!({
      "type": "object",
      "properties": {
        "message": { "type": "string" }
      }
    }),
  }
}

/// Create a test LockedComponent with typed schema for coercion tests.
fn test_locked_component_with_typed_schema() -> LockedComponent {
  LockedComponent {
    name: "test-task".to_string(),
    version: "1.0.0".to_string(),
    digest: "sha256:abc123".to_string(),
    task_name: "execute".to_string(),
    input_schema: json!({
      "type": "object",
      "properties": {
        "count": { "type": "integer" },
        "enabled": { "type": "boolean" },
        "name": { "type": "string" }
      }
    }),
  }
}

/// Create a manual trigger node.
fn create_manual_trigger(node_id: &str) -> Node {
  Node {
    node_id: node_id.to_string(),
    node_type: NodeType::Trigger(LockedTrigger {
      trigger_type: fuscia_component::TriggerType::Manual,
      component: None,
    }),
    inputs: HashMap::new(),
    timeout_ms: None,
    max_retry_attempts: None,
    fail_workflow: true,
  }
}

/// Create a simple workflow with a trigger node and one task node.
fn create_simple_workflow() -> Workflow {
  let mut nodes = HashMap::new();

  // Trigger node (entry point)
  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  // Task node
  let mut task_inputs = HashMap::new();
  task_inputs.insert("message".to_string(), "{{ message }}".to_string());

  nodes.insert(
    "process".to_string(),
    Node {
      node_id: "process".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: task_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  Workflow {
    workflow_id: "test-workflow".to_string(),
    name: "Test Workflow".to_string(),
    nodes,
    edges: vec![("trigger".to_string(), "process".to_string())],
    timeout_ms: None,
    max_retry_attempts: None,
  }
}

/// Create a workflow with two parallel branches that converge at a join node.
fn create_parallel_workflow() -> Workflow {
  let mut nodes = HashMap::new();

  // Trigger node
  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  // Branch A - processes "a" field
  let mut branch_a_inputs = HashMap::new();
  branch_a_inputs.insert("message".to_string(), "{{ a }}".to_string());

  nodes.insert(
    "branch_a".to_string(),
    Node {
      node_id: "branch_a".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: branch_a_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  // Branch B - processes "b" field
  let mut branch_b_inputs = HashMap::new();
  branch_b_inputs.insert("message".to_string(), "{{ b }}".to_string());

  nodes.insert(
    "branch_b".to_string(),
    Node {
      node_id: "branch_b".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: branch_b_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  // Join node
  nodes.insert(
    "join".to_string(),
    Node {
      node_id: "join".to_string(),
      node_type: NodeType::Join {
        strategy: fuscia_config::JoinStrategy::All,
      },
      inputs: HashMap::new(),
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  Workflow {
    workflow_id: "parallel-workflow".to_string(),
    name: "Parallel Workflow".to_string(),
    nodes,
    edges: vec![
      ("trigger".to_string(), "branch_a".to_string()),
      ("trigger".to_string(), "branch_b".to_string()),
      ("branch_a".to_string(), "join".to_string()),
      ("branch_b".to_string(), "join".to_string()),
    ],
    timeout_ms: None,
    max_retry_attempts: None,
  }
}

#[tokio::test]
async fn test_simple_workflow_execution() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  let workflow = create_simple_workflow();
  let payload = json!({ "message": "hello world" });
  let cancel = CancellationToken::new();

  let result = engine
    .execute(&workflow, payload, cancel)
    .await
    .expect("workflow execution failed");

  // Should have executed trigger and process nodes
  assert_eq!(result.node_results.len(), 2);
  assert!(result.node_results.contains_key("trigger"));
  assert!(result.node_results.contains_key("process"));

  // Check the process node output
  let process_result = &result.node_results["process"];
  let output: serde_json::Value =
    serde_json::from_value(process_result.data.clone()).expect("invalid output");

  // The test component wraps the input in { "input": {...} }
  assert_eq!(output["input"]["message"], "hello world");
  assert_eq!(output["stored_matches"], true);
}

#[tokio::test]
async fn test_parallel_workflow_execution() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  let workflow = create_parallel_workflow();
  let payload = json!({ "a": "value_a", "b": "value_b" });
  let cancel = CancellationToken::new();

  let result = engine
    .execute(&workflow, payload, cancel)
    .await
    .expect("workflow execution failed");

  // Should have executed all 4 nodes
  assert_eq!(result.node_results.len(), 4);
  assert!(result.node_results.contains_key("trigger"));
  assert!(result.node_results.contains_key("branch_a"));
  assert!(result.node_results.contains_key("branch_b"));
  assert!(result.node_results.contains_key("join"));

  // Check branch_a output
  let branch_a = &result.node_results["branch_a"];
  let output_a: serde_json::Value =
    serde_json::from_value(branch_a.data.clone()).expect("invalid output");
  assert_eq!(output_a["input"]["message"], "value_a");

  // Check branch_b output
  let branch_b = &result.node_results["branch_b"];
  let output_b: serde_json::Value =
    serde_json::from_value(branch_b.data.clone()).expect("invalid output");
  assert_eq!(output_b["input"]["message"], "value_b");

  // Check join node merged both branches
  let join_result = &result.node_results["join"];
  assert!(join_result.data.get("branch_a").is_some());
  assert!(join_result.data.get("branch_b").is_some());
}

#[tokio::test]
async fn test_workflow_cancellation() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  let workflow = create_simple_workflow();
  let payload = json!({ "message": "test" });
  let cancel = CancellationToken::new();

  // Cancel immediately
  cancel.cancel();

  let result = engine.execute(&workflow, payload, cancel).await;

  assert!(matches!(result, Err(ExecutionError::Cancelled)));
}

#[tokio::test]
async fn test_workflow_runner() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = Arc::new(WorkflowEngine::new(config).expect("failed to create engine"));

  let workflow = create_simple_workflow();
  let runner = WorkflowRunner::new(workflow, engine);

  // Execute once directly
  let cancel = CancellationToken::new();
  let result = runner
    .execute_once(json!({ "message": "runner test" }), cancel)
    .await
    .expect("execution failed");

  assert_eq!(result.node_results.len(), 2);

  let process_result = &result.node_results["process"];
  let output: serde_json::Value =
    serde_json::from_value(process_result.data.clone()).expect("invalid output");
  assert_eq!(output["input"]["message"], "runner test");
}

#[tokio::test]
async fn test_workflow_runner_with_sender() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = Arc::new(WorkflowEngine::new(config).expect("failed to create engine"));

  let workflow = create_simple_workflow();
  let runner = WorkflowRunner::new(workflow, engine);
  let sender = runner.sender();

  let cancel = CancellationToken::new();
  let cancel_clone = cancel.clone();

  // Start runner in background
  let handle = tokio::spawn(async move { runner.start(cancel_clone).await });

  // Send a payload through the channel
  sender
    .send(json!({ "message": "from sender" }))
    .await
    .expect("send failed");

  // Give it a moment to process
  tokio::time::sleep(std::time::Duration::from_millis(100)).await;

  // Cancel the runner
  cancel.cancel();

  // Runner should exit cleanly
  handle.await.expect("join failed").expect("runner failed");
}

#[tokio::test]
async fn test_input_template_resolution() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  // Create a workflow where the task input uses a filter
  let mut nodes = HashMap::new();

  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  let mut task_inputs = HashMap::new();
  task_inputs.insert("message".to_string(), "{{ name | upper }}".to_string());

  nodes.insert(
    "process".to_string(),
    Node {
      node_id: "process".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: task_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  let workflow = Workflow {
    workflow_id: "filter-test".to_string(),
    name: "Filter Test".to_string(),
    nodes,
    edges: vec![("trigger".to_string(), "process".to_string())],
    timeout_ms: None,
    max_retry_attempts: None,
  };

  let payload = json!({ "name": "hello" });
  let cancel = CancellationToken::new();

  let result = engine
    .execute(&workflow, payload, cancel)
    .await
    .expect("execution failed");

  let process_result = &result.node_results["process"];
  let output: serde_json::Value =
    serde_json::from_value(process_result.data.clone()).expect("invalid output");

  // The minijinja `upper` filter should have been applied
  assert_eq!(output["input"]["message"], "HELLO");
}

#[tokio::test]
async fn test_orphan_node_validation() {
  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  // Create a workflow with an orphan node (no incoming edges, not a trigger)
  let mut nodes = HashMap::new();

  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  // Orphan node - Component with no incoming edges
  nodes.insert(
    "orphan".to_string(),
    Node {
      node_id: "orphan".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: HashMap::new(),
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  // Task node connected to trigger
  nodes.insert(
    "process".to_string(),
    Node {
      node_id: "process".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: HashMap::new(),
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  let workflow = Workflow {
    workflow_id: "orphan-test".to_string(),
    name: "Orphan Test".to_string(),
    nodes,
    // Note: "orphan" has no incoming edge
    edges: vec![("trigger".to_string(), "process".to_string())],
    timeout_ms: None,
    max_retry_attempts: None,
  };

  let payload = json!({});
  let cancel = CancellationToken::new();

  let result = engine.execute(&workflow, payload, cancel).await;

  assert!(matches!(result, Err(ExecutionError::InvalidGraph { .. })));
  if let Err(ExecutionError::InvalidGraph { message }) = result {
    assert!(message.contains("orphan"));
  }
}

#[tokio::test]
async fn test_multiple_triggers() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  // Create a workflow with two triggers
  let mut nodes = HashMap::new();

  nodes.insert(
    "webhook_trigger".to_string(),
    create_manual_trigger("webhook_trigger"),
  );
  nodes.insert(
    "cron_trigger".to_string(),
    create_manual_trigger("cron_trigger"),
  );

  // Task node that both triggers feed into via a join
  nodes.insert(
    "join".to_string(),
    Node {
      node_id: "join".to_string(),
      node_type: NodeType::Join {
        strategy: fuscia_config::JoinStrategy::Any,
      },
      inputs: HashMap::new(),
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  let mut task_inputs = HashMap::new();
  task_inputs.insert("data".to_string(), "{{ webhook_trigger }}".to_string());

  nodes.insert(
    "process".to_string(),
    Node {
      node_id: "process".to_string(),
      node_type: NodeType::Component(test_locked_component()),
      inputs: task_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  let workflow = Workflow {
    workflow_id: "multi-trigger".to_string(),
    name: "Multi-Trigger Workflow".to_string(),
    nodes,
    edges: vec![
      ("webhook_trigger".to_string(), "join".to_string()),
      ("cron_trigger".to_string(), "join".to_string()),
      ("join".to_string(), "process".to_string()),
    ],
    timeout_ms: None,
    max_retry_attempts: None,
  };

  let payload = json!({ "source": "test" });
  let cancel = CancellationToken::new();

  let result = engine
    .execute(&workflow, payload, cancel)
    .await
    .expect("workflow execution failed");

  // Should have executed all nodes
  assert_eq!(result.node_results.len(), 4);
  assert!(result.node_results.contains_key("webhook_trigger"));
  assert!(result.node_results.contains_key("cron_trigger"));
  assert!(result.node_results.contains_key("join"));
  assert!(result.node_results.contains_key("process"));
}

#[tokio::test]
async fn test_schema_type_coercion_success() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  // Create a workflow with typed inputs
  let mut nodes = HashMap::new();

  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  let mut task_inputs = HashMap::new();
  task_inputs.insert("count".to_string(), "{{ count }}".to_string());
  task_inputs.insert("enabled".to_string(), "{{ enabled }}".to_string());
  task_inputs.insert("name".to_string(), "{{ name }}".to_string());

  nodes.insert(
    "process".to_string(),
    Node {
      node_id: "process".to_string(),
      node_type: NodeType::Component(test_locked_component_with_typed_schema()),
      inputs: task_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  let workflow = Workflow {
    workflow_id: "coercion-test".to_string(),
    name: "Coercion Test".to_string(),
    nodes,
    edges: vec![("trigger".to_string(), "process".to_string())],
    timeout_ms: None,
    max_retry_attempts: None,
  };

  // Pass values that will be coerced: "42" -> integer, "true" -> boolean
  let payload = json!({ "count": 42, "enabled": true, "name": "test" });
  let cancel = CancellationToken::new();

  let result = engine
    .execute(&workflow, payload, cancel)
    .await
    .expect("workflow execution failed");

  let process_result = &result.node_results["process"];
  let output: serde_json::Value =
    serde_json::from_value(process_result.data.clone()).expect("invalid output");

  // Verify the inputs were coerced to correct types
  assert_eq!(output["input"]["count"], 42);
  assert_eq!(output["input"]["enabled"], true);
  assert_eq!(output["input"]["name"], "test");
}

#[tokio::test]
async fn test_schema_type_coercion_failure() {
  if !component_exists() {
    eprintln!(
      "Skipping test: test component not built. Run `cargo component build --release` in test-components/test-task-component"
    );
    return;
  }

  let (config, _temp_dir) = create_test_engine_config();
  let engine = WorkflowEngine::new(config).expect("failed to create engine");

  // Create a workflow with typed inputs
  let mut nodes = HashMap::new();

  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  let mut task_inputs = HashMap::new();
  // This will try to coerce "not_a_number" to an integer, which should fail
  task_inputs.insert("count".to_string(), "{{ count }}".to_string());
  task_inputs.insert("enabled".to_string(), "true".to_string());
  task_inputs.insert("name".to_string(), "test".to_string());

  nodes.insert(
    "process".to_string(),
    Node {
      node_id: "process".to_string(),
      node_type: NodeType::Component(test_locked_component_with_typed_schema()),
      inputs: task_inputs,
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

  let workflow = Workflow {
    workflow_id: "coercion-fail-test".to_string(),
    name: "Coercion Failure Test".to_string(),
    nodes,
    edges: vec![("trigger".to_string(), "process".to_string())],
    timeout_ms: None,
    max_retry_attempts: None,
  };

  // Pass an invalid value: "not_a_number" cannot be coerced to integer
  let payload = json!({ "count": "not_a_number" });
  let cancel = CancellationToken::new();

  let result = engine.execute(&workflow, payload, cancel).await;

  // Should fail with an InputResolution error
  assert!(matches!(
    result,
    Err(ExecutionError::InputResolution { .. })
  ));
  if let Err(ExecutionError::InputResolution { node_id, message }) = result {
    assert_eq!(node_id, "process");
    assert!(message.contains("count") || message.contains("integer"));
  }
}
