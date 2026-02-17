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
  /// Mirrors LMDBLiteCache.setLargeBlob() in JavaScript - stores data in
  /// `{cache_dir}/files/{sanitized_key}` to avoid bloating LMDB with large values.
  fn set_large_blob(&self, key: &str, blob: &[u8]) -> anyhow::Result<()>;

  /// Get a large blob from the filesystem cache.
  fn get_large_blob(&self, key: &str) -> anyhow::Result<Vec<u8>>;
}

/// Convert a cache key to a safe filesystem path.
/// Mirrors the logic in JavaScript's LMDBLiteCache.getFileKey() -
/// sanitizes `..` in path segments to avoid directory traversal.
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
  cache_dir.join("files").join(clean_key)
}

/// A cache implementation backed by LMDB for small blobs and the filesystem for large blobs.
/// Mirrors the behaviour of LMDBLiteCache in JavaScript.
pub struct LmdbCache {
  db: Arc<lmdb_js_lite::DatabaseHandle>,
  cache_dir: PathBuf,
}

impl LmdbCache {
  pub fn new(db: Arc<lmdb_js_lite::DatabaseHandle>, cache_dir: PathBuf) -> Self {
    Self { db, cache_dir }
  }
}

impl Cache for LmdbCache {
  fn set_blob(&self, key: &str, blob: &[u8]) -> anyhow::Result<()> {
    let mut write_txn = self.db.database().write_txn()?;
    self.db.database().put(&mut write_txn, key, blob)?;
    write_txn.commit()?;
    Ok(())
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
