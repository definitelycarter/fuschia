//! Workflow runtime.
//!
//! The [`Runtime`] struct is the main entry point for executing workflows.
//! It owns a workflow, wasmtime engine, and component cache, and provides
//! `invoke(payload, cancel)` to execute the full workflow graph.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use fuschia_workflow::{LockedComponent, NodeType, Workflow};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};
use wasmtime::Engine;
use wasmtime::component::Component;

use crate::cache::{ComponentCache, ComponentKey};
use crate::error::RuntimeError;
use crate::input::{coerce_inputs, extract_schema_types, resolve_inputs};
use crate::result::{InvokeResult, NodeResult};
use crate::task::{TaskExecutor, TaskInput};
use crate::trigger::{TriggerExecutor, TriggerInput};
use fuschia_trigger_host::Status;

/// Handle for a spawned node task.
type NodeHandle = tokio::task::JoinHandle<Result<NodeResult, RuntimeError>>;

/// Result of trigger execution.
enum TriggerOutcome {
  /// Trigger completed — proceed with graph execution.
  Continue(NodeResult),
  /// Trigger returned Pending — short-circuit, don't run the graph.
  ShortCircuit(NodeResult),
}

/// Configuration for the runtime.
pub struct RuntimeConfig {
  /// Base path for resolving component wasm files.
  pub component_base_path: PathBuf,
}

/// The workflow runtime.
///
/// Handles graph traversal, scheduling, template resolution, and orchestrates
/// component execution via [`TaskExecutor`].
pub struct Runtime {
  workflow: Workflow,
  engine: Arc<Engine>,
  component_cache: ComponentCache,
  config: RuntimeConfig,
}

impl Runtime {
  /// Create a new runtime for the given workflow.
  pub fn new(workflow: Workflow, config: RuntimeConfig) -> Result<Self, RuntimeError> {
    let mut wasmtime_config = wasmtime::Config::new();
    wasmtime_config.async_support(true);
    wasmtime_config.epoch_interruption(true);

    let engine =
      Arc::new(
        Engine::new(&wasmtime_config).map_err(|e| RuntimeError::InvalidGraph {
          message: format!("failed to create wasmtime engine: {}", e),
        })?,
      );

    Ok(Self {
      workflow,
      engine,
      component_cache: ComponentCache::new(),
      config,
    })
  }

  /// Execute the workflow with the given trigger payload.
  #[instrument(
    name = "runtime_invoke",
    skip(self, payload, cancel),
    fields(
      workflow_id = %self.workflow.workflow_id,
    )
  )]
  pub async fn invoke(
    &self,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<InvokeResult, RuntimeError> {
    let execution_id = uuid::Uuid::new_v4().to_string();

    info!(
      execution_id = %execution_id,
      workflow_id = %self.workflow.workflow_id,
      trigger_payload = %payload,
      "workflow_started"
    );

    // Validate the workflow graph
    self.validate_workflow()?;

    // Execute the trigger — run its wasm component if present
    let trigger_result = self
      .execute_trigger(payload, &execution_id, &cancel)
      .await?;

    let mut completed = match trigger_result {
      TriggerOutcome::Continue(node_result) => {
        HashMap::from([(node_result.node_id.clone(), node_result)])
      }
      TriggerOutcome::ShortCircuit(node_result) => {
        info!(execution_id = %execution_id, "workflow_completed (trigger short-circuit)");
        return Ok(InvokeResult {
          execution_id,
          node_results: HashMap::from([(node_result.node_id.clone(), node_result)]),
        });
      }
    };

    // Execute nodes until no more are ready
    let result = self
      .run_execution_loop(&mut completed, &execution_id, &cancel)
      .await;

    match &result {
      Ok(_) => {
        info!(execution_id = %execution_id, "workflow_completed");
      }
      Err(e) => {
        error!(execution_id = %execution_id, error = %e, "workflow_failed");
      }
    }

    result
  }

  /// Execute a single node in isolation.
  ///
  /// This is for debugging — run one Component or Trigger node without
  /// walking the graph. The payload is used as the upstream data context
  /// for input resolution.
  ///
  /// Only `Component` and `Trigger` nodes are supported. `Join` and `Loop`
  /// nodes are graph-level concerns and return an error.
  #[instrument(
    name = "runtime_invoke_node",
    skip(self, payload, cancel),
    fields(
      workflow_id = %self.workflow.workflow_id,
      node_id = %node_id,
    )
  )]
  pub async fn invoke_node(
    &self,
    node_id: &str,
    payload: serde_json::Value,
    cancel: CancellationToken,
  ) -> Result<NodeResult, RuntimeError> {
    let node = self
      .workflow
      .get_node(node_id)
      .ok_or_else(|| RuntimeError::InvalidGraph {
        message: format!("node '{}' not found in workflow", node_id),
      })?
      .clone();

    let execution_id = uuid::Uuid::new_v4().to_string();
    let task_id = uuid::Uuid::new_v4().to_string();

    info!(
      execution_id = %execution_id,
      task_id = %task_id,
      node_id = %node_id,
      payload = %payload,
      "invoke_node_started"
    );

    let result = match &node.node_type {
      NodeType::Component(locked) => {
        // Treat payload as single upstream data for template resolution
        let mut upstream_data = HashMap::new();
        upstream_data.insert("_payload".to_string(), payload.clone());

        let resolved_strings = resolve_inputs(&node.node_id, &node.inputs, &upstream_data, false)?;
        let schema = extract_schema_types(&locked.input_schema);
        let resolved_input = coerce_inputs(&node.node_id, &resolved_strings, &schema)?;

        let component = self.load_component(locked)?;

        let task_input = TaskInput {
          task_id,
          execution_id,
          node_id: node_id.to_string(),
          input: payload,
          resolved_input,
          timeout_ms: node.timeout_ms,
        };

        let task_executor = TaskExecutor::new(self.engine.clone());
        task_executor.execute(&component, task_input, cancel).await
      }
      NodeType::Trigger(locked_trigger) => {
        match &locked_trigger.component {
          Some(trigger_component) => {
            let component = self.load_trigger_component(trigger_component)?;

            let event =
              fuschia_trigger_host::Event::Webhook(fuschia_trigger_host::WebhookRequest {
                method: "POST".to_string(),
                path: String::new(),
                headers: Vec::new(),
                body: Some(serde_json::to_string(&payload).unwrap_or_default()),
              });

            let trigger_input = TriggerInput {
              execution_id,
              node_id: node_id.to_string(),
              event,
              timeout_ms: node.timeout_ms,
            };

            let trigger_executor = TriggerExecutor::new(self.engine.clone());
            let trigger_result = trigger_executor
              .execute(&component, trigger_input, cancel)
              .await?;

            Ok(NodeResult {
              task_id,
              node_id: node_id.to_string(),
              input: payload,
              resolved_input: serde_json::Value::Null,
              output: serde_json::json!({
                "status": format!("{:?}", trigger_result.status),
              }),
            })
          }
          None => {
            // No component — pass through the payload
            Ok(NodeResult {
              task_id,
              node_id: node_id.to_string(),
              input: serde_json::Value::Null,
              resolved_input: serde_json::Value::Null,
              output: payload,
            })
          }
        }
      }
      NodeType::Join { .. } => Err(RuntimeError::InvalidGraph {
        message: format!(
          "cannot invoke_node on join node '{}' — joins are graph-level concerns",
          node_id
        ),
      }),
      NodeType::Loop(_) => Err(RuntimeError::InvalidGraph {
        message: format!(
          "cannot invoke_node on loop node '{}' — loops are graph-level concerns",
          node_id
        ),
      }),
    };

    match &result {
      Ok(node_result) => {
        info!(
          node_id = %node_id,
          output = %node_result.output,
          "invoke_node_completed"
        );
      }
      Err(e) => {
        error!(node_id = %node_id, error = %e, "invoke_node_failed");
      }
    }

    result
  }

  /// Get a reference to the workflow.
  pub fn workflow(&self) -> &Workflow {
    &self.workflow
  }

  /// Run the main execution loop.
  async fn run_execution_loop(
    &self,
    completed: &mut HashMap<String, NodeResult>,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<InvokeResult, RuntimeError> {
    loop {
      if cancel.is_cancelled() {
        warn!(execution_id = %execution_id, "workflow cancelled");
        return Err(RuntimeError::Cancelled);
      }

      let ready = self.find_ready_nodes(completed);
      if ready.is_empty() {
        break;
      }

      info!(
        execution_id = %execution_id,
        ready_nodes = ?ready,
        "executing batch of ready nodes"
      );

      let handles = self.execute_ready_nodes(&ready, completed, execution_id, cancel)?;

      // Wait for all tasks
      let results = tokio::select! {
        results = futures::future::join_all(handles) => results,
        _ = cancel.cancelled() => {
          warn!(execution_id = %execution_id, "workflow cancelled during task execution");
          return Err(RuntimeError::Cancelled);
        }
      };

      // Process results
      for result in results {
        let node_result = result
          .map_err(|e| RuntimeError::InvalidGraph {
            message: format!("task join error: {}", e),
          })?
          .map_err(|e| {
            error!(execution_id = %execution_id, error = %e, "task_failed");
            e
          })?;

        info!(
          execution_id = %execution_id,
          task_id = %node_result.task_id,
          node_id = %node_result.node_id,
          output = %node_result.output,
          "task_completed"
        );
        completed.insert(node_result.node_id.clone(), node_result);
      }
    }

    Ok(InvokeResult {
      execution_id: execution_id.to_string(),
      node_results: completed.clone(),
    })
  }

  /// Execute the trigger node's wasm component (if present).
  ///
  /// Returns `TriggerOutcome::Continue` if the workflow should proceed
  /// (component returned Completed, or no component), or
  /// `TriggerOutcome::ShortCircuit` if the trigger returned Pending.
  async fn execute_trigger(
    &self,
    payload: serde_json::Value,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<TriggerOutcome, RuntimeError> {
    // validate_workflow already enforces exactly one trigger
    let trigger_id = self.find_trigger_nodes().into_iter().next().unwrap();
    let node = self.workflow.get_node(&trigger_id).unwrap().clone();
    let locked_trigger = match &node.node_type {
      NodeType::Trigger(lt) => lt.clone(),
      _ => unreachable!(),
    };

    let task_id = uuid::Uuid::new_v4().to_string();

    match &locked_trigger.component {
      Some(trigger_component) => {
        // Execute the trigger wasm component
        let component = self.load_trigger_component(trigger_component)?;

        let event = match &locked_trigger.trigger_type {
          fuschia_config::TriggerType::Poll { .. } => fuschia_trigger_host::Event::Poll,
          fuschia_config::TriggerType::Webhook { method, .. } => {
            fuschia_trigger_host::Event::Webhook(fuschia_trigger_host::WebhookRequest {
              method: method.clone(),
              path: String::new(),
              headers: Vec::new(),
              body: Some(serde_json::to_string(&payload).unwrap_or_default()),
            })
          }
          fuschia_config::TriggerType::Manual => {
            fuschia_trigger_host::Event::Webhook(fuschia_trigger_host::WebhookRequest {
              method: "POST".to_string(),
              path: String::new(),
              headers: Vec::new(),
              body: Some(serde_json::to_string(&payload).unwrap_or_default()),
            })
          }
        };

        let trigger_input = TriggerInput {
          execution_id: execution_id.to_string(),
          node_id: trigger_id.clone(),
          event,
          timeout_ms: node.timeout_ms,
        };

        let trigger_executor = TriggerExecutor::new(self.engine.clone());
        let trigger_result = trigger_executor
          .execute(&component, trigger_input, cancel.clone())
          .await?;

        match trigger_result.status {
          Status::Completed(data) => {
            let output: serde_json::Value =
              serde_json::from_str(&data).unwrap_or(serde_json::Value::String(data));
            info!(
              execution_id = %execution_id,
              node_id = %trigger_id,
              "trigger component returned completed"
            );
            Ok(TriggerOutcome::Continue(NodeResult {
              task_id,
              node_id: trigger_id,
              input: payload,
              resolved_input: serde_json::Value::Null,
              output,
            }))
          }
          Status::Pending => {
            info!(
              execution_id = %execution_id,
              node_id = %trigger_id,
              "trigger component returned pending, short-circuiting"
            );
            Ok(TriggerOutcome::ShortCircuit(NodeResult {
              task_id,
              node_id: trigger_id,
              input: payload,
              resolved_input: serde_json::Value::Null,
              output: serde_json::Value::Null,
            }))
          }
        }
      }
      None => {
        // No component — pass through the payload directly
        Ok(TriggerOutcome::Continue(NodeResult {
          task_id,
          node_id: trigger_id,
          input: payload.clone(),
          resolved_input: serde_json::Value::Null,
          output: payload,
        }))
      }
    }
  }

  /// Load and compile a trigger component, using the cache if available.
  fn load_trigger_component(
    &self,
    trigger_component: &fuschia_workflow::LockedTriggerComponent,
  ) -> Result<Component, RuntimeError> {
    let key = ComponentKey::new(&trigger_component.name, &trigger_component.version);
    let sanitized_name = trigger_component.name.replace('/', "--");
    let dir_name = format!("{}--{}", sanitized_name, trigger_component.version);
    let wasm_path = self
      .config
      .component_base_path
      .join(&dir_name)
      .join("component.wasm");
    self
      .component_cache
      .get_or_compile(&self.engine, &key, &wasm_path)
  }

  /// Find nodes that are ready to execute (all upstream nodes completed).
  fn find_ready_nodes(&self, completed: &HashMap<String, NodeResult>) -> Vec<String> {
    let graph = self.workflow.graph();

    self
      .workflow
      .nodes
      .keys()
      .filter(|id| !completed.contains_key(*id))
      .filter(|id| {
        graph
          .upstream(id)
          .iter()
          .all(|up| completed.contains_key(up))
      })
      .cloned()
      .collect()
  }

  /// Spawn tasks to execute all ready nodes in parallel.
  fn execute_ready_nodes(
    &self,
    ready: &[String],
    completed: &HashMap<String, NodeResult>,
    execution_id: &str,
    cancel: &CancellationToken,
  ) -> Result<Vec<NodeHandle>, RuntimeError> {
    let graph = self.workflow.graph();
    let mut handles = Vec::with_capacity(ready.len());

    for node_id in ready {
      let node = self
        .workflow
        .get_node(node_id)
        .ok_or_else(|| RuntimeError::InvalidGraph {
          message: format!("node '{}' not found in workflow", node_id),
        })?
        .clone();
      let upstream_ids: Vec<String> = graph.upstream(node_id).to_vec();
      let is_join = graph.is_join_point(node_id);

      // Gather upstream data (using output from completed nodes)
      let upstream_data: HashMap<String, serde_json::Value> = upstream_ids
        .iter()
        .filter_map(|id| completed.get(id).map(|r| (id.clone(), r.output.clone())))
        .collect();

      // Generate task_id
      let task_id = uuid::Uuid::new_v4().to_string();

      // Build input from upstream data
      let input = if is_join {
        serde_json::Value::Object(
          upstream_data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        )
      } else if upstream_data.len() == 1 {
        upstream_data
          .values()
          .next()
          .cloned()
          .unwrap_or(serde_json::Value::Null)
      } else {
        serde_json::Value::Null
      };

      // Resolve inputs (template rendering + type coercion)
      let resolved_input = match &node.node_type {
        NodeType::Component(locked) => {
          let resolved_strings =
            resolve_inputs(&node.node_id, &node.inputs, &upstream_data, is_join)?;
          let schema = extract_schema_types(&locked.input_schema);
          let resolved = coerce_inputs(&node.node_id, &resolved_strings, &schema)?;
          serde_json::to_value(&resolved).unwrap_or(serde_json::Value::Null)
        }
        NodeType::Join { .. } => input.clone(),
        _ => serde_json::Value::Null,
      };

      info!(
        execution_id = %execution_id,
        task_id = %task_id,
        node_id = %node_id,
        input = %input,
        resolved_input = %resolved_input,
        "task_started"
      );

      let task_input = TaskInput {
        task_id,
        execution_id: execution_id.to_string(),
        node_id: node_id.clone(),
        input,
        resolved_input,
        timeout_ms: node.timeout_ms,
      };

      // Load component if needed
      let component = match &node.node_type {
        NodeType::Component(locked) => Some(self.load_component(locked)?),
        _ => None,
      };

      let engine = self.engine.clone();
      let cancel = cancel.clone();

      handles.push(tokio::spawn(async move {
        match node.node_type {
          NodeType::Component(_) => {
            let Some(component) = component else {
              return Err(RuntimeError::InvalidGraph {
                message: "component not loaded for component node".to_string(),
              });
            };
            let task_executor = TaskExecutor::new(engine);
            task_executor.execute(&component, task_input, cancel).await
          }
          NodeType::Join { .. } => Ok(execute_join(task_input)),
          NodeType::Trigger(_) => Err(RuntimeError::InvalidGraph {
            message: "trigger nodes should not be executed".to_string(),
          }),
          NodeType::Loop(_) => Err(RuntimeError::InvalidGraph {
            message: "loop nodes not yet implemented".to_string(),
          }),
        }
      }));
    }

    Ok(handles)
  }

  /// Load and compile a component, using the cache if available.
  fn load_component(&self, locked: &LockedComponent) -> Result<Component, RuntimeError> {
    let key = ComponentKey::new(&locked.name, &locked.version);
    let sanitized_name = locked.name.replace('/', "--");
    let dir_name = format!("{}--{}", sanitized_name, locked.version);
    let wasm_path = self
      .config
      .component_base_path
      .join(&dir_name)
      .join("component.wasm");
    self
      .component_cache
      .get_or_compile(&self.engine, &key, &wasm_path)
  }

  /// Find all trigger nodes in the workflow.
  fn find_trigger_nodes(&self) -> Vec<String> {
    self
      .workflow
      .nodes
      .iter()
      .filter(|(_, node)| matches!(node.node_type, NodeType::Trigger(_)))
      .map(|(id, _)| id.clone())
      .collect()
  }

  /// Validate the workflow graph.
  fn validate_workflow(&self) -> Result<(), RuntimeError> {
    let graph = self.workflow.graph();
    let entry_points = graph.entry_points();

    // Enforce single trigger
    let trigger_nodes = self.find_trigger_nodes();
    if trigger_nodes.len() != 1 {
      return Err(RuntimeError::InvalidGraph {
        message: format!(
          "workflow must have exactly one trigger, found {}",
          trigger_nodes.len()
        ),
      });
    }

    for entry_id in entry_points {
      let node = self
        .workflow
        .get_node(entry_id)
        .ok_or_else(|| RuntimeError::InvalidGraph {
          message: format!("entry point '{}' not found in nodes", entry_id),
        })?;

      if !matches!(node.node_type, NodeType::Trigger(_)) {
        return Err(RuntimeError::InvalidGraph {
          message: format!(
            "node '{}' has no incoming edges but is not a trigger (orphan node)",
            entry_id
          ),
        });
      }
    }

    Ok(())
  }
}

/// Execute a join node (merges upstream inputs, no wasm).
fn execute_join(input: TaskInput) -> NodeResult {
  info!(
    task_id = %input.task_id,
    node_id = %input.node_id,
    input = %input.input,
    "join completed"
  );

  NodeResult {
    task_id: input.task_id,
    node_id: input.node_id,
    input: input.input.clone(),
    resolved_input: input.resolved_input,
    output: input.input,
  }
}
