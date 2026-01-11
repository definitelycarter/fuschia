//! Fuscia Artifact
//!
//! This crate provides the artifact storage trait and implementations for Fuscia.
//! Artifacts are binary blobs (images, files, large data) that are stored separately
//! from workflow execution data.
//!
//! The [`Store`] trait defines the platform/backend layer for artifact storage.
//! Implementations handle the actual storage (filesystem, S3, etc.) while the
//! engine translates artifact IDs to storage keys.
//!
//! The trait uses async streaming for efficient handling of large files.

mod fs;

pub use fs::FsStore;

use bytes::Bytes;
use futures::Stream;

/// Artifact storage trait.
///
/// Implementations provide the actual storage backend (filesystem, S3, etc.).
/// The engine is responsible for translating artifact IDs to storage keys.
pub trait Store {
  /// Error type for storage operations.
  type Error;

  /// Stream type returned by `get`.
  type GetStream: Stream<Item = Result<Bytes, Self::Error>>;

  /// Retrieve an artifact by key.
  ///
  /// Returns a stream of bytes for efficient handling of large files.
  fn get(
    &self,
    key: &str,
  ) -> impl std::future::Future<Output = Result<Self::GetStream, Self::Error>> + Send;

  /// Store an artifact.
  ///
  /// Accepts a stream of bytes for efficient handling of large files.
  fn put<S>(
    &self,
    key: &str,
    data: S,
    content_type: &str,
  ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send
  where
    S: Stream<Item = Result<Bytes, Self::Error>> + Send;

  /// Delete an artifact by key.
  fn delete(&self, key: &str) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}
