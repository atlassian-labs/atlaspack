use std::sync::Arc;

use atlaspack_core::database_reader::DatabaseReader;
use lmdb_js_lite::DatabaseHandle;

/// A `DatabaseReader` implementation backed by an LMDB `DatabaseHandle`.
pub struct LmdbDatabaseReader(pub Arc<DatabaseHandle>);

impl DatabaseReader for LmdbDatabaseReader {
  fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let txn = self.0.database().read_txn()?;
    let result = self.0.database().get(&txn, key)?;
    txn.commit()?;
    Ok(result)
  }
}
