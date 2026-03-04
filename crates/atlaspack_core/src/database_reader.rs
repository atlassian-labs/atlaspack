use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

pub type DatabaseReaderRef = Arc<dyn DatabaseReader + Send + Sync>;

/// Abstracts over database read operations.
///
/// The primary production implementation wraps `lmdb_js_lite::DatabaseHandle`.
/// An in-memory implementation (`InMemoryDatabaseReader`) is provided for use in tests.
pub trait DatabaseReader: std::fmt::Debug {
  fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
}

/// An in-memory `DatabaseReader` suitable for use in tests.
///
/// Use `put` to seed test data before running code under test.
#[derive(Debug, Default)]
pub struct InMemoryDatabaseReader {
  data: RwLock<HashMap<String, Vec<u8>>>,
}

impl InMemoryDatabaseReader {
  pub fn put(&self, key: &str, value: Vec<u8>) {
    self.data.write().insert(key.to_string(), value);
  }
}

impl DatabaseReader for InMemoryDatabaseReader {
  fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    Ok(self.data.read().get(key).cloned())
  }
}
