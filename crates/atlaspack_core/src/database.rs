use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

pub type DatabaseRef = Arc<dyn Database + Send + Sync>;

/// Abstracts over database read and write operations.
///
/// The primary production implementation wraps `lmdb_js_lite::DatabaseHandle`.
/// An in-memory implementation ([`InMemoryDatabase`]) is provided for use in tests.
pub trait Database: std::fmt::Debug {
  fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;
}

/// An in-memory [`Database`] suitable for use in tests.
///
/// Use [`put`](Database::put) to seed test data before running code under test.
#[derive(Debug, Default)]
pub struct InMemoryDatabase {
  data: RwLock<HashMap<String, Vec<u8>>>,
}

impl Database for InMemoryDatabase {
  fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    Ok(self.data.read().get(key).cloned())
  }

  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    self.data.write().insert(key.to_string(), value.to_vec());
    Ok(())
  }
}
