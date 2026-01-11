use std::io;
use std::path::PathBuf;

use bytes::Bytes;
use futures::{Stream, StreamExt};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

use crate::Store;

/// Filesystem-based artifact store.
///
/// Stores artifacts as files on the local filesystem. Each artifact is stored
/// at `{base_path}/{key}`. Parent directories are created automatically.
pub struct FsStore {
  base_path: PathBuf,
}

impl FsStore {
  /// Create a new filesystem store with the given base path.
  pub fn new(base_path: impl Into<PathBuf>) -> Self {
    Self {
      base_path: base_path.into(),
    }
  }

  fn key_to_path(&self, key: &str) -> PathBuf {
    self.base_path.join(key)
  }
}

impl Store for FsStore {
  type Error = io::Error;
  type GetStream = ReaderStream<File>;

  async fn get(&self, key: &str) -> Result<Self::GetStream, Self::Error> {
    let path = self.key_to_path(key);
    let file = File::open(path).await?;
    Ok(ReaderStream::new(file))
  }

  async fn put<S>(&self, key: &str, data: S, _content_type: &str) -> Result<(), Self::Error>
  where
    S: Stream<Item = Result<Bytes, Self::Error>> + Send,
  {
    let path = self.key_to_path(key);

    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).await?;
    }

    let mut file = File::create(path).await?;
    let mut stream = std::pin::pin!(data);

    while let Some(chunk) = stream.next().await {
      let bytes = chunk?;
      file.write_all(&bytes).await?;
    }

    file.flush().await?;
    Ok(())
  }

  async fn delete(&self, key: &str) -> Result<(), Self::Error> {
    let path = self.key_to_path(key);
    fs::remove_file(path).await
  }
}
