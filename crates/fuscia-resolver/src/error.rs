use thiserror::Error;

/// Errors that can occur during workflow resolution.
#[derive(Debug, Error)]
pub enum ResolveError {
  /// Component not found in the registry.
  #[error("component not found: {name}")]
  ComponentNotFound { name: String },

  /// Component version not found.
  #[error("component version not found: {name}@{version}")]
  ComponentVersionNotFound { name: String, version: String },

  /// Invalid edge reference (node doesn't exist).
  #[error("invalid edge: node '{node_id}' does not exist")]
  InvalidEdge { node_id: String },

  /// Duplicate node ID.
  #[error("duplicate node id: {node_id}")]
  DuplicateNodeId { node_id: String },

  /// Workflow has no entry points (no nodes without incoming edges).
  #[error("workflow has no entry points")]
  NoEntryPoints,

  /// Cycle detected in the workflow graph.
  #[error("cycle detected in workflow graph")]
  CycleDetected,

  /// Invalid trigger configuration.
  #[error("invalid trigger '{node_id}': {message}")]
  InvalidTrigger { node_id: String, message: String },

  /// Registry error while looking up component.
  #[error("registry error: {0}")]
  Registry(#[from] fuscia_component::RegistryError),
}
