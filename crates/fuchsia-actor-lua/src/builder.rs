use crate::actor::LuaActor;
use crate::host::LuaHost;
use fuchsia_actor::ActorError;
use std::path::PathBuf;
use std::sync::Arc;

/// Builds a [`LuaActor`]. Acquired via [`LuaActor::builder`].
///
/// A script source is required (one of `source`, `source_from_path`).
pub struct LuaActorBuilder<H: LuaHost> {
  host: H,
  source: Option<SourceKind>,
}

enum SourceKind {
  Inline(String),
  Path(PathBuf),
}

impl<H: LuaHost> LuaActorBuilder<H> {
  pub(crate) fn new(host: H) -> Self {
    Self { host, source: None }
  }

  /// Provide the Lua script source directly.
  pub fn source(mut self, source: impl Into<String>) -> Self {
    self.source = Some(SourceKind::Inline(source.into()));
    self
  }

  /// Load the Lua script source from a file on disk.
  pub fn source_from_path(mut self, path: impl Into<PathBuf>) -> Self {
    self.source = Some(SourceKind::Path(path.into()));
    self
  }

  pub fn build(self) -> Result<LuaActor<H>, ActorError> {
    let source = match self.source {
      Some(SourceKind::Inline(s)) => s,
      Some(SourceKind::Path(p)) => std::fs::read_to_string(&p)
        .map_err(|e| ActorError::Other(format!("read lua source {}: {e}", p.display())))?,
      None => {
        return Err(ActorError::Other(
          "LuaActorBuilder requires a source (source or source_from_path)".into(),
        ));
      }
    };

    Ok(LuaActor {
      source: Arc::new(source),
      host: Arc::new(self.host),
    })
  }
}
