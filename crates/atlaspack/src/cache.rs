use std::sync::Arc;

use atlaspack_core::cache::{Cache, get_file_key};
use atlaspack_filesystem::FileSystemRef;
use lmdb_js_lite::DatabaseHandle;

/// A cache implementation backed by LMDB for small blobs and the filesystem for large blobs.
/// Mirrors the behaviour of LMDBLiteCache in JavaScript.
pub struct LmdbCache {
  db: Arc<DatabaseHandle>,
  cache_dir: std::path::PathBuf,
  fs: FileSystemRef,
}

impl LmdbCache {
  pub fn new(db: Arc<DatabaseHandle>, fs: FileSystemRef) -> Self {
    // The LMDB database is opened at the cache directory path, so we can derive
    // cache_dir directly from the database handle. This avoids requiring cache_dir
    // to be threaded through AtlaspackOptions separately.
    let cache_dir = db.database().path().to_path_buf();
    Self { db, cache_dir, fs }
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
      self.fs.create_dir_all(parent)?;
    }
    self.fs.write(&file_path, blob)?;
    Ok(())
  }

  fn get_large_blob(&self, key: &str) -> anyhow::Result<Vec<u8>> {
    let file_path = get_file_key(&self.cache_dir, key);
    Ok(self.fs.read(&file_path)?)
  }
}
