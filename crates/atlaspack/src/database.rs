use std::fmt;
use std::sync::Arc;

use atlaspack_core::database::Database;
use lmdb_js_lite::DatabaseHandle;

/// A [`Database`] implementation backed by an LMDB [`DatabaseHandle`].
pub struct LmdbDatabase(pub Arc<DatabaseHandle>);

impl fmt::Debug for LmdbDatabase {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LmdbDatabase").finish_non_exhaustive()
  }
}

impl Database for LmdbDatabase {
  fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let txn = self.0.database().read_txn()?;
    let result = self.0.database().get(&txn, key)?;
    txn.commit()?;
    Ok(result)
  }

  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    let mut txn = self.0.database().write_txn()?;
    self.0.database().put(&mut txn, key, value)?;
    txn.commit()?;
    Ok(())
  }
}
