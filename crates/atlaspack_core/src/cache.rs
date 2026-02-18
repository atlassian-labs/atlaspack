use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub type CacheRef = Arc<dyn Cache + Sync + Send>;

#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
#[mockall::automock]
pub trait Cache {
  fn set_blob(&self, _key: &str, _blob: &[u8]) -> anyhow::Result<()>;

  /// Write a large blob to the filesystem cache.
  /// Mirrors LMDBLiteCache:
  /// path is `{cache_dir}/{sanitized_key}` so JS getStream(path.join(this.dir, key)) finds it.
  fn set_large_blob(&self, key: &str, blob: &[u8]) -> anyhow::Result<()>;

  /// Get a large blob from the filesystem cache.
  fn get_large_blob(&self, key: &str) -> anyhow::Result<Vec<u8>>;
}

/// Convert a cache key to a safe filesystem path.
/// Sanitizes `..` in path segments to avoid directory traversal.
pub fn get_file_key(cache_dir: &Path, key: &str) -> PathBuf {
  let clean_key = key
    .split('/')
    .map(|part| {
      if part == ".." {
        "$$__parent_dir$$"
      } else {
        part
      }
    })
    .collect::<Vec<_>>()
    .join("/");
  cache_dir.join(clean_key)
}

/// A filesystem-only cache implementation.
/// Stores all blobs on the filesystem under `{cache_dir}/{key}`.
pub struct FsCache {
  cache_dir: PathBuf,
}

impl FsCache {
  pub fn new(cache_dir: PathBuf) -> Self {
    Self { cache_dir }
  }
}

impl Cache for FsCache {
  fn set_blob(&self, key: &str, blob: &[u8]) -> anyhow::Result<()> {
    self.set_large_blob(key, blob)
  }

  fn set_large_blob(&self, key: &str, blob: &[u8]) -> anyhow::Result<()> {
    let file_path = get_file_key(&self.cache_dir, key);
    if let Some(parent) = file_path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(&file_path, blob)?;
    Ok(())
  }

  fn get_large_blob(&self, key: &str) -> anyhow::Result<Vec<u8>> {
    let file_path = get_file_key(&self.cache_dir, key);
    Ok(fs::read(&file_path)?)
  }
}
