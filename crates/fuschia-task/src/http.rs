use std::collections::HashMap;

use reqwest::{Client, Method};
use serde::Deserialize;

use crate::error::TaskError;
use crate::types::{TaskContext, TaskOutput};

/// Expected input schema for HTTP tasks.
#[derive(Debug, Deserialize)]
struct HttpInput {
  method: String,
  url: String,
  #[serde(default)]
  headers: HashMap<String, String>,
  #[serde(default)]
  body: Option<serde_json::Value>,
}

/// Execute an HTTP request task.
pub async fn execute(ctx: &TaskContext) -> Result<TaskOutput, TaskError> {
  // Parse inputs
  let input: HttpInput =
    serde_json::from_value(ctx.inputs.clone()).map_err(|e| TaskError::InvalidInput {
      field: "inputs".to_string(),
      message: e.to_string(),
    })?;

  // Parse method
  let method = parse_method(&input.method)?;

  // Build request
  let client = Client::new();
  let mut request = client.request(method, &input.url);

  // Add headers
  for (key, value) in &input.headers {
    request = request.header(key, value);
  }

  // Add body if present
  if let Some(body) = &input.body {
    request = request.json(body);
  }

  // Execute request
  let response = request.send().await?;

  // Build output
  let status = response.status().as_u16();
  let headers: HashMap<String, String> = response
    .headers()
    .iter()
    .filter_map(|(k, v)| {
      v.to_str()
        .ok()
        .map(|val| (k.as_str().to_string(), val.to_string()))
    })
    .collect();

  let body = response.text().await?;

  // Try to parse body as JSON, fall back to string
  let body_value = serde_json::from_str(&body).unwrap_or(serde_json::Value::String(body));

  let output = serde_json::json!({
      "status": status,
      "headers": headers,
      "body": body_value,
  });

  Ok(TaskOutput {
    output,
    artifacts: vec![],
  })
}

fn parse_method(method: &str) -> Result<Method, TaskError> {
  match method.to_uppercase().as_str() {
    "GET" => Ok(Method::GET),
    "POST" => Ok(Method::POST),
    "PUT" => Ok(Method::PUT),
    "DELETE" => Ok(Method::DELETE),
    "PATCH" => Ok(Method::PATCH),
    "HEAD" => Ok(Method::HEAD),
    "OPTIONS" => Ok(Method::OPTIONS),
    _ => Err(TaskError::InvalidInput {
      field: "method".to_string(),
      message: format!("unsupported HTTP method: {}", method),
    }),
  }
}
