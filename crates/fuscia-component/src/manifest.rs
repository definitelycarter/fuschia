use serde::{Deserialize, Serialize};

/// Metadata describing an installed component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentManifest {
  /// Component name, e.g. "my-org/sentiment-analysis"
  pub name: String,

  /// Semantic version, e.g. "1.0.0"
  pub version: String,

  /// Short description of what the component does
  pub description: String,

  /// SHA-256 digest of the wasm binary, e.g. "sha256:abc123..."
  pub digest: String,

  /// JSON Schema defining the expected input structure
  pub input_schema: serde_json::Value,
}

impl ComponentManifest {
  /// Returns the directory name for this component: "name--version"
  /// with slashes replaced by double dashes.
  ///
  /// Example: "my-org/sentiment-analysis" version "1.0.0"
  /// becomes "my-org--sentiment-analysis--1.0.0"
  pub fn dir_name(&self) -> String {
    let sanitized_name = self.name.replace('/', "--");
    format!("{}--{}", sanitized_name, self.version)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_dir_name() {
    let manifest = ComponentManifest {
      name: "my-org/sentiment-analysis".to_string(),
      version: "1.0.0".to_string(),
      description: "Analyzes sentiment".to_string(),
      digest: "sha256:abc123".to_string(),
      input_schema: serde_json::json!({}),
    };

    assert_eq!(manifest.dir_name(), "my-org--sentiment-analysis--1.0.0");
  }

  #[test]
  fn test_dir_name_no_org() {
    let manifest = ComponentManifest {
      name: "simple-component".to_string(),
      version: "2.1.0".to_string(),
      description: "A simple component".to_string(),
      digest: "sha256:def456".to_string(),
      input_schema: serde_json::json!({}),
    };

    assert_eq!(manifest.dir_name(), "simple-component--2.1.0");
  }
}
