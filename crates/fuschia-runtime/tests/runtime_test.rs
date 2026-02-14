//! Integration tests for Runtime::invoke using real wasm components.

use std::collections::HashMap;
use std::path::PathBuf;

use fuschia_config::{JoinStrategy, TriggerType};
use fuschia_runtime::{Runtime, RuntimeConfig, RuntimeError};
use fuschia_workflow::{LockedComponent, LockedTrigger, Node, NodeType, Workflow};
use serde_json::json;
use tokio_util::sync::CancellationToken;

/// Path to the test task component.
const TEST_COMPONENT_PATH: &str =
  "../../test-components/test-task-component/target/wasm32-wasip1/release/test_task_component.wasm";

fn test_component_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TEST_COMPONENT_PATH)
}

fn component_exists() -> bool {
  test_component_path().exists()
}

/// Create a RuntimeConfig that points to a temp directory where we symlink the test component.
fn create_test_config() -> (RuntimeConfig, tempfile::TempDir) {
  let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

  let component_dir = temp_dir.path().join("test-task--1.0.0");
  std::fs::create_dir_all(&component_dir).expect("failed to create component dir");

  let src = test_component_path();
  let dst = component_dir.join("component.wasm");

  #[cfg(unix)]
  std::os::unix::fs::symlink(&src, &dst).expect("failed to symlink component");

  #[cfg(windows)]
  std::fs::copy(&src, &dst).expect("failed to copy component");

  let config = RuntimeConfig {
    component_base_path: temp_dir.path().to_path_buf(),
  };

  (config, temp_dir)
}

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

fn create_manual_trigger(node_id: &str) -> Node {
  Node {
    node_id: node_id.to_string(),
    node_type: NodeType::Trigger(LockedTrigger {
      trigger_type: TriggerType::Manual,
      component: None,
    }),
    inputs: HashMap::new(),
    timeout_ms: None,
    max_retry_attempts: None,
    fail_workflow: true,
  }
}

fn create_simple_workflow() -> Workflow {
  let mut nodes = HashMap::new();

  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

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

fn create_parallel_workflow() -> Workflow {
  let mut nodes = HashMap::new();

  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

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

  nodes.insert(
    "join".to_string(),
    Node {
      node_id: "join".to_string(),
      node_type: NodeType::Join {
        strategy: JoinStrategy::All,
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
async fn test_simple_invoke() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({ "message": "hello world" });
  let cancel = CancellationToken::new();

  let result = runtime
    .invoke(payload, cancel)
    .await
    .expect("invoke failed");

  assert_eq!(result.node_results.len(), 2);
  assert!(result.node_results.contains_key("trigger"));
  assert!(result.node_results.contains_key("process"));

  let process_result = &result.node_results["process"];
  assert_eq!(process_result.output["input"]["message"], "hello world");
  assert_eq!(process_result.output["stored_matches"], true);
}

#[tokio::test]
async fn test_parallel_branches_with_join() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();
  let workflow = create_parallel_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({ "a": "value_a", "b": "value_b" });
  let cancel = CancellationToken::new();

  let result = runtime
    .invoke(payload, cancel)
    .await
    .expect("invoke failed");

  assert_eq!(result.node_results.len(), 4);
  assert!(result.node_results.contains_key("trigger"));
  assert!(result.node_results.contains_key("branch_a"));
  assert!(result.node_results.contains_key("branch_b"));
  assert!(result.node_results.contains_key("join"));

  let branch_a = &result.node_results["branch_a"];
  assert_eq!(branch_a.output["input"]["message"], "value_a");

  let branch_b = &result.node_results["branch_b"];
  assert_eq!(branch_b.output["input"]["message"], "value_b");

  let join_result = &result.node_results["join"];
  assert!(join_result.output.get("branch_a").is_some());
  assert!(join_result.output.get("branch_b").is_some());
}

#[tokio::test]
async fn test_invoke_cancellation() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({ "message": "test" });
  let cancel = CancellationToken::new();
  cancel.cancel();

  let result = runtime.invoke(payload, cancel).await;
  assert!(matches!(result, Err(RuntimeError::Cancelled)));
}

#[tokio::test]
async fn test_input_template_resolution() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();

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

  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");
  let payload = json!({ "name": "hello" });
  let cancel = CancellationToken::new();

  let result = runtime
    .invoke(payload, cancel)
    .await
    .expect("invoke failed");

  let process_result = &result.node_results["process"];
  assert_eq!(process_result.output["input"]["message"], "HELLO");
}

#[tokio::test]
async fn test_type_coercion() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();

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

  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");
  let payload = json!({ "count": 42, "enabled": true, "name": "test" });
  let cancel = CancellationToken::new();

  let result = runtime
    .invoke(payload, cancel)
    .await
    .expect("invoke failed");

  let process_result = &result.node_results["process"];
  assert_eq!(process_result.output["input"]["count"], 42);
  assert_eq!(process_result.output["input"]["enabled"], true);
  assert_eq!(process_result.output["input"]["name"], "test");
}

#[tokio::test]
async fn test_type_coercion_failure() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();

  let mut nodes = HashMap::new();
  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

  let mut task_inputs = HashMap::new();
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

  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");
  let payload = json!({ "count": "not_a_number" });
  let cancel = CancellationToken::new();

  let result = runtime.invoke(payload, cancel).await;

  assert!(matches!(result, Err(RuntimeError::InputResolution { .. })));
  if let Err(RuntimeError::InputResolution { node_id, .. }) = result {
    assert_eq!(node_id, "process");
  }
}

#[tokio::test]
async fn test_orphan_node_validation() {
  let (config, _temp_dir) = create_test_config();

  let mut nodes = HashMap::new();
  nodes.insert("trigger".to_string(), create_manual_trigger("trigger"));

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
    edges: vec![("trigger".to_string(), "process".to_string())],
    timeout_ms: None,
    max_retry_attempts: None,
  };

  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");
  let payload = json!({});
  let cancel = CancellationToken::new();

  let result = runtime.invoke(payload, cancel).await;

  assert!(matches!(result, Err(RuntimeError::InvalidGraph { .. })));
  if let Err(RuntimeError::InvalidGraph { message }) = result {
    assert!(message.contains("orphan"));
  }
}

#[tokio::test]
async fn test_multiple_triggers_rejected() {
  let (config, _temp_dir) = create_test_config();

  let mut nodes = HashMap::new();
  nodes.insert(
    "webhook_trigger".to_string(),
    create_manual_trigger("webhook_trigger"),
  );
  nodes.insert(
    "cron_trigger".to_string(),
    create_manual_trigger("cron_trigger"),
  );

  nodes.insert(
    "join".to_string(),
    Node {
      node_id: "join".to_string(),
      node_type: NodeType::Join {
        strategy: JoinStrategy::Any,
      },
      inputs: HashMap::new(),
      timeout_ms: None,
      max_retry_attempts: None,
      fail_workflow: true,
    },
  );

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

  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");
  let payload = json!({ "source": "test" });
  let cancel = CancellationToken::new();

  let result = runtime.invoke(payload, cancel).await;
  assert!(matches!(result, Err(RuntimeError::InvalidGraph { .. })));
  if let Err(RuntimeError::InvalidGraph { message }) = result {
    assert!(message.contains("exactly one trigger"));
  }
}

#[tokio::test]
async fn test_workflow_accessor() {
  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  assert_eq!(runtime.workflow().workflow_id, "test-workflow");
  assert_eq!(runtime.workflow().name, "Test Workflow");
}

// --- invoke_node tests ---

#[tokio::test]
async fn test_invoke_node_component() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({ "message": "hello from invoke_node" });
  let cancel = CancellationToken::new();

  let result = runtime
    .invoke_node("process", payload, cancel)
    .await
    .expect("invoke_node failed");

  assert_eq!(result.node_id, "process");
  assert_eq!(result.output["input"]["message"], "hello from invoke_node");
}

#[tokio::test]
async fn test_invoke_node_trigger_no_component() {
  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({ "source": "manual" });
  let cancel = CancellationToken::new();

  let result = runtime
    .invoke_node("trigger", payload.clone(), cancel)
    .await
    .expect("invoke_node failed");

  assert_eq!(result.node_id, "trigger");
  // Trigger without component passes through the payload
  assert_eq!(result.output, payload);
}

#[tokio::test]
async fn test_invoke_node_join_errors() {
  let (config, _temp_dir) = create_test_config();
  let workflow = create_parallel_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({});
  let cancel = CancellationToken::new();

  let result = runtime.invoke_node("join", payload, cancel).await;

  assert!(matches!(result, Err(RuntimeError::InvalidGraph { .. })));
  if let Err(RuntimeError::InvalidGraph { message }) = result {
    assert!(message.contains("join"));
    assert!(message.contains("graph-level"));
  }
}

#[tokio::test]
async fn test_invoke_node_not_found() {
  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({});
  let cancel = CancellationToken::new();

  let result = runtime.invoke_node("nonexistent", payload, cancel).await;

  assert!(matches!(result, Err(RuntimeError::InvalidGraph { .. })));
  if let Err(RuntimeError::InvalidGraph { message }) = result {
    assert!(message.contains("nonexistent"));
  }
}

#[tokio::test]
async fn test_invoke_node_cancellation() {
  if !component_exists() {
    eprintln!("Skipping test: test component not built");
    return;
  }

  let (config, _temp_dir) = create_test_config();
  let workflow = create_simple_workflow();
  let runtime = Runtime::new(workflow, config).expect("failed to create runtime");

  let payload = json!({ "message": "test" });
  let cancel = CancellationToken::new();
  cancel.cancel();

  let result = runtime.invoke_node("process", payload, cancel).await;
  assert!(matches!(result, Err(RuntimeError::Cancelled)));
}
