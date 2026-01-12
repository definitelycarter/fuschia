//! Input resolution using minijinja templates.
//!
//! Resolves node inputs by rendering minijinja templates against upstream node data.
//!
//! # Single Upstream Node
//! When a node has one upstream, the context is the upstream node's `data` directly:
//! ```json
//! { "recipient": "{{ email }}", "greeting": "Hello {{ name | title }}!" }
//! ```
//!
//! # Join Node (Multiple Upstream)
//! When a node has multiple upstream nodes, context is keyed by node name:
//! ```json
//! { "user_email": "{{ fetch_user.email }}", "config": "{{ get_config.setting }}" }
//! ```

use std::collections::HashMap;

use fuscia_config::InputValue;
use minijinja::{Environment, Value};
use serde_json::json;

use crate::error::ExecutionError;

/// Resolve a node's inputs against upstream data.
///
/// # Arguments
/// * `node_id` - The ID of the node being resolved (for error messages)
/// * `inputs` - The node's input definitions (may contain templates)
/// * `upstream_data` - Data from upstream nodes, keyed by node_id
/// * `is_join` - Whether this is a join node (multiple upstream)
pub fn resolve_inputs(
  node_id: &str,
  inputs: &HashMap<String, InputValue>,
  upstream_data: &HashMap<String, serde_json::Value>,
  is_join: bool,
) -> Result<serde_json::Value, ExecutionError> {
  let env = Environment::new();

  // Build the context based on whether this is a join node
  let context = if is_join {
    // Join node: context is keyed by upstream node names
    // { "fetch_user": {...}, "get_config": {...} }
    let ctx: serde_json::Map<String, serde_json::Value> = upstream_data
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect();
    serde_json::Value::Object(ctx)
  } else {
    // Single upstream: context is the upstream data directly
    // { "email": "...", "name": "..." }
    upstream_data.values().next().cloned().unwrap_or(json!({}))
  };

  let ctx_value = Value::from_serialize(&context);

  let mut resolved = serde_json::Map::new();

  for (key, input_value) in inputs {
    let value = resolve_input_value(&env, node_id, key, input_value, &ctx_value)?;
    resolved.insert(key.clone(), value);
  }

  Ok(serde_json::Value::Object(resolved))
}

/// Resolve a single input value.
fn resolve_input_value(
  env: &Environment,
  node_id: &str,
  input_key: &str,
  input_value: &InputValue,
  context: &Value,
) -> Result<serde_json::Value, ExecutionError> {
  match input_value {
    InputValue::Literal(value) => {
      // Literal values may contain templates in string fields
      resolve_json_value(env, node_id, input_key, value, context)
    }
    InputValue::Path(path) => {
      // Path references like "upstream.field" - resolve against context
      let template = format!("{{{{ {} }}}}", path);
      let result = render_template(env, node_id, input_key, &template, context)?;
      // Try to parse as JSON, fall back to string
      let value: serde_json::Value =
        serde_json::from_str(&result).unwrap_or(serde_json::Value::String(result));
      Ok(value)
    }
  }
}

/// Recursively resolve templates in a JSON value.
fn resolve_json_value(
  env: &Environment,
  node_id: &str,
  input_key: &str,
  value: &serde_json::Value,
  context: &Value,
) -> Result<serde_json::Value, ExecutionError> {
  match value {
    serde_json::Value::String(s) => {
      // Check if the string contains template syntax
      if s.contains("{{") || s.contains("{%") {
        let result = render_template(env, node_id, input_key, s, context)?;
        // If the entire string was a template, try to parse the result as JSON
        // This allows templates like "{{ count }}" to resolve to numbers
        if is_pure_template(s) {
          if let Ok(parsed) = serde_json::from_str(&result) {
            return Ok(parsed);
          }
        }
        Ok(serde_json::Value::String(result))
      } else {
        Ok(value.clone())
      }
    }
    serde_json::Value::Array(arr) => {
      let resolved: Result<Vec<_>, _> = arr
        .iter()
        .map(|v| resolve_json_value(env, node_id, input_key, v, context))
        .collect();
      Ok(serde_json::Value::Array(resolved?))
    }
    serde_json::Value::Object(obj) => {
      let mut resolved = serde_json::Map::new();
      for (k, v) in obj {
        resolved.insert(
          k.clone(),
          resolve_json_value(env, node_id, input_key, v, context)?,
        );
      }
      Ok(serde_json::Value::Object(resolved))
    }
    // Numbers, bools, nulls pass through unchanged
    _ => Ok(value.clone()),
  }
}

/// Check if a string is a pure template expression (just `{{ expr }}`).
fn is_pure_template(s: &str) -> bool {
  let trimmed = s.trim();
  trimmed.starts_with("{{")
    && trimmed.ends_with("}}")
    && trimmed.matches("{{").count() == 1
    && trimmed.matches("}}").count() == 1
}

/// Render a minijinja template.
fn render_template(
  env: &Environment,
  node_id: &str,
  input_key: &str,
  template: &str,
  context: &Value,
) -> Result<String, ExecutionError> {
  env
    .render_str(template, context.clone())
    .map_err(|e| ExecutionError::InputResolution {
      node_id: node_id.to_string(),
      message: format!("failed to resolve input '{}': {}", input_key, e),
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_resolve_single_upstream() {
    let mut inputs = HashMap::new();
    inputs.insert(
      "recipient".to_string(),
      InputValue::Literal(json!("{{ email }}")),
    );
    inputs.insert(
      "greeting".to_string(),
      InputValue::Literal(json!("Hello {{ name | title }}!")),
    );

    let mut upstream_data = HashMap::new();
    upstream_data.insert(
      "fetch_user".to_string(),
      json!({
          "email": "test@example.com",
          "name": "john doe"
      }),
    );

    let result = resolve_inputs("send_email", &inputs, &upstream_data, false).unwrap();

    assert_eq!(result["recipient"], "test@example.com");
    assert_eq!(result["greeting"], "Hello John Doe!");
  }

  #[test]
  fn test_resolve_join_node() {
    let mut inputs = HashMap::new();
    inputs.insert(
      "user_email".to_string(),
      InputValue::Literal(json!("{{ fetch_user.email }}")),
    );
    inputs.insert(
      "config_value".to_string(),
      InputValue::Literal(json!("{{ get_config.setting }}")),
    );

    let mut upstream_data = HashMap::new();
    upstream_data.insert(
      "fetch_user".to_string(),
      json!({ "email": "test@example.com" }),
    );
    upstream_data.insert("get_config".to_string(), json!({ "setting": "enabled" }));

    let result = resolve_inputs("process", &inputs, &upstream_data, true).unwrap();

    assert_eq!(result["user_email"], "test@example.com");
    assert_eq!(result["config_value"], "enabled");
  }

  #[test]
  fn test_resolve_literal_number() {
    let mut inputs = HashMap::new();
    inputs.insert("count".to_string(), InputValue::Literal(json!(42)));
    inputs.insert(
      "from_template".to_string(),
      InputValue::Literal(json!("{{ count }}")),
    );

    let mut upstream_data = HashMap::new();
    upstream_data.insert("prev".to_string(), json!({ "count": 100 }));

    let result = resolve_inputs("node", &inputs, &upstream_data, false).unwrap();

    assert_eq!(result["count"], 42); // Literal passes through
    assert_eq!(result["from_template"], 100); // Template resolves to number
  }

  #[test]
  fn test_resolve_nested_object() {
    let mut inputs = HashMap::new();
    inputs.insert(
      "config".to_string(),
      InputValue::Literal(json!({
          "email": "{{ email }}",
          "settings": {
              "name": "{{ name }}"
          }
      })),
    );

    let mut upstream_data = HashMap::new();
    upstream_data.insert(
      "prev".to_string(),
      json!({
          "email": "test@example.com",
          "name": "Test User"
      }),
    );

    let result = resolve_inputs("node", &inputs, &upstream_data, false).unwrap();

    assert_eq!(result["config"]["email"], "test@example.com");
    assert_eq!(result["config"]["settings"]["name"], "Test User");
  }

  #[test]
  fn test_resolve_path_input() {
    let mut inputs = HashMap::new();
    inputs.insert(
      "email".to_string(),
      InputValue::Path("user.email".to_string()),
    );

    let mut upstream_data = HashMap::new();
    upstream_data.insert(
      "prev".to_string(),
      json!({
          "user": {
              "email": "test@example.com"
          }
      }),
    );

    let result = resolve_inputs("node", &inputs, &upstream_data, false).unwrap();

    assert_eq!(result["email"], "test@example.com");
  }

  #[test]
  fn test_empty_upstream() {
    let mut inputs = HashMap::new();
    inputs.insert("static".to_string(), InputValue::Literal(json!("hello")));

    let upstream_data = HashMap::new();

    let result = resolve_inputs("node", &inputs, &upstream_data, false).unwrap();

    assert_eq!(result["static"], "hello");
  }

  #[test]
  fn test_minijinja_filters() {
    let mut inputs = HashMap::new();
    inputs.insert(
      "upper".to_string(),
      InputValue::Literal(json!("{{ name | upper }}")),
    );
    inputs.insert(
      "lower".to_string(),
      InputValue::Literal(json!("{{ name | lower }}")),
    );

    let mut upstream_data = HashMap::new();
    upstream_data.insert("prev".to_string(), json!({ "name": "Test" }));

    let result = resolve_inputs("node", &inputs, &upstream_data, false).unwrap();

    assert_eq!(result["upper"], "TEST");
    assert_eq!(result["lower"], "test");
  }
}
