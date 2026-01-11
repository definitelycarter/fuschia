use serde::{Deserialize, Serialize};

/// Source location for a Wasm component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComponentSource {
  /// Load from a local file path.
  File { path: String },
  /// Load from a URL.
  Url {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
  },
}
