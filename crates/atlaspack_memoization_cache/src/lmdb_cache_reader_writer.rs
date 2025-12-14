use crossbeam::queue::SegQueue;
use lmdb_js_lite::DatabaseHandle;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::CacheReaderWriter;

pub struct LmdbCacheReaderWriter {
  db_handle: Arc<DatabaseHandle>,
  entries: SegQueue<String>,
  max_entries: usize,
}

impl LmdbCacheReaderWriter {
  pub fn new(db_handle: Arc<DatabaseHandle>) -> Self {
    Self {
      db_handle,
      entries: SegQueue::new(),
      max_entries: 300000, // Default max entries
    }
  }

  pub fn with_max_entries(db_handle: Arc<DatabaseHandle>, max_entries: usize) -> Self {
    Self {
      db_handle,
      entries: SegQueue::new(),
      max_entries,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct CacheEntry {
  timestamp: u64,
  key: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheTracker {
  entries: BTreeSet<CacheEntry>,
  key_to_timestamp: HashMap<String, u64>,
}

impl CacheTracker {
  fn update_entry(&mut self, key: String, timestamp: u64) {
    // Remove old entry if exists
    if let Some(old_timestamp) = self.key_to_timestamp.get(&key) {
      let old_entry = CacheEntry {
        timestamp: *old_timestamp,
        key: key.clone(),
      };
      self.entries.remove(&old_entry);
    }

    // Insert new entry
    let new_entry = CacheEntry {
      timestamp,
      key: key.clone(),
    };
    self.entries.insert(new_entry);
    self.key_to_timestamp.insert(key, timestamp);
  }

  fn remove_oldest_entries(&mut self, count: usize) -> Vec<String> {
    let keys_to_remove: Vec<String> = self
      .entries
      .iter()
      .take(count)
      .map(|entry| entry.key.clone())
      .collect();

    for key in &keys_to_remove {
      self.remove_key(key);
    }

    keys_to_remove
  }

  fn remove_key(&mut self, key: &str) {
    if let Some(timestamp) = self.key_to_timestamp.remove(key) {
      let entry = CacheEntry {
        timestamp,
        key: key.to_string(),
      };
      self.entries.remove(&entry);
    }
  }
}

impl CacheReaderWriter for LmdbCacheReaderWriter {
  #[tracing::instrument(level = "trace", skip_all)]
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let db_writer = self.db_handle.database();
    let transaction = db_writer.read_txn()?;

    // Don't track metadata reads as session keys
    if key != "memo_cache_metadata" {
      self.entries.push(key.to_string());
    }

    db_writer
      .get(&transaction, key)
      .map_err(|error| anyhow::anyhow!("Read transaction error: {}", error))
  }

  #[tracing::instrument(level = "trace", skip_all)]
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    let db_writer = self.db_handle.database();
    let mut transaction = db_writer.write_txn()?;

    // Don't track metadata writes as session keys
    if key != "memo_cache_metadata" {
      self.entries.push(key.to_string());
    }
    db_writer.put(&mut transaction, key, value)?;

    transaction
      .commit()
      .map_err(|error| anyhow::anyhow!("Put commit error: {}", error))
  }

  #[tracing::instrument(level = "info", skip_all)]
  fn complete_session(&self) -> anyhow::Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // Load existing metadata
    let mut cache_tracker = if let Some(raw_metadata) = self.read("memo_cache_metadata")? {
      bincode::deserialize::<CacheTracker>(&raw_metadata)?
    } else {
      CacheTracker::default()
    };

    // Update entries for all keys from this session
    while let Some(key) = self.entries.pop() {
      cache_tracker.update_entry(key, now);
    }

    // Enforce max entries limit
    let entries_to_remove = cache_tracker.entries.len().saturating_sub(self.max_entries);
    if entries_to_remove > 0 {
      tracing::info!(
        "Cache exceeded max entries ({}). Removing {} oldest entries.",
        self.max_entries,
        entries_to_remove
      );
      let removed_keys = cache_tracker.remove_oldest_entries(entries_to_remove);

      // Optionally remove the actual cache entries from LMDB
      for key in removed_keys {
        let db_writer = self.db_handle.database();
        let mut transaction = db_writer.write_txn()?;
        let _ = db_writer.delete(&mut transaction, &key);
        transaction.commit().ok(); // Ignore errors on cleanup
      }
    }

    // Save updated metadata back to database
    let serialized_metadata = bincode::serialize(&cache_tracker)?;
    self.put("memo_cache_metadata", &serialized_metadata)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use lmdb_js_lite::DatabaseHandle;
  use pretty_assertions::assert_eq;
  use std::sync::Arc;
  use tempfile::tempdir;

  fn create_test_db() -> Arc<DatabaseHandle> {
    use lmdb_js_lite::{LMDBOptions, get_database};
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };
    get_database(options).unwrap()
  }

  #[test]
  fn test_cache_tracker_new() {
    let tracker = CacheTracker::default();
    assert_eq!(tracker.entries.len(), 0);
    assert_eq!(tracker.key_to_timestamp.len(), 0);
  }

  #[test]
  fn test_cache_tracker_update_entry() {
    let mut tracker = CacheTracker::default();

    tracker.update_entry("key1".to_string(), 1000);
    assert_eq!(tracker.entries.len(), 1);
    assert_eq!(tracker.key_to_timestamp.len(), 1);
    assert_eq!(tracker.key_to_timestamp.get("key1"), Some(&1000));

    // Update same key with new timestamp
    tracker.update_entry("key1".to_string(), 2000);
    assert_eq!(tracker.entries.len(), 1);
    assert_eq!(tracker.key_to_timestamp.len(), 1);
    assert_eq!(tracker.key_to_timestamp.get("key1"), Some(&2000));
  }

  #[test]
  fn test_cache_tracker_multiple_entries() {
    let mut tracker = CacheTracker::default();

    tracker.update_entry("key1".to_string(), 1000);
    tracker.update_entry("key2".to_string(), 2000);
    tracker.update_entry("key3".to_string(), 1500);

    assert_eq!(tracker.entries.len(), 3);

    // Check ordering (oldest first)
    let keys: Vec<String> = tracker.entries.iter().map(|e| e.key.clone()).collect();
    assert_eq!(keys, vec!["key1", "key3", "key2"]);
  }

  #[test]
  fn test_cache_tracker_remove_key() {
    let mut tracker = CacheTracker::default();

    tracker.update_entry("key1".to_string(), 1000);
    tracker.update_entry("key2".to_string(), 2000);

    tracker.remove_key("key1");
    assert_eq!(tracker.entries.len(), 1);
    assert_eq!(tracker.key_to_timestamp.len(), 1);
    assert!(!tracker.key_to_timestamp.contains_key("key1"));
    assert!(tracker.key_to_timestamp.contains_key("key2"));
  }

  #[test]
  fn test_cache_tracker_remove_oldest_entries() {
    let mut tracker = CacheTracker::default();

    // Add entries with different timestamps
    tracker.update_entry("oldest".to_string(), 1000);
    tracker.update_entry("middle".to_string(), 2000);
    tracker.update_entry("newest".to_string(), 3000);

    let removed = tracker.remove_oldest_entries(2);

    assert_eq!(removed.len(), 2);
    assert!(removed.contains(&"oldest".to_string()));
    assert!(removed.contains(&"middle".to_string()));
    assert!(!removed.contains(&"newest".to_string()));

    assert_eq!(tracker.entries.len(), 1);
    assert_eq!(tracker.key_to_timestamp.len(), 1);
    assert!(tracker.key_to_timestamp.contains_key("newest"));
  }

  #[test]
  fn test_cache_tracker_serialization() {
    let mut tracker = CacheTracker::default();
    tracker.update_entry("key1".to_string(), 1000);
    tracker.update_entry("key2".to_string(), 2000);

    let serialized = bincode::serialize(&tracker).unwrap();
    let deserialized: CacheTracker = bincode::deserialize(&serialized).unwrap();

    assert_eq!(tracker.entries.len(), deserialized.entries.len());
    assert_eq!(tracker.key_to_timestamp, deserialized.key_to_timestamp);
  }

  #[test]
  fn test_put_and_read() {
    let db = create_test_db();
    let cache = LmdbCacheReaderWriter::new(db);

    let key = "test_key";
    let value = b"test_value";

    cache.put(key, value).unwrap();
    let result = cache.read(key).unwrap();

    assert_eq!(result, Some(value.to_vec()));
  }

  #[test]
  fn test_read_nonexistent_key() {
    let db = create_test_db();
    let cache = LmdbCacheReaderWriter::new(db);

    let result = cache.read("nonexistent").unwrap();
    assert_eq!(result, None);
  }

  #[test]
  fn test_session_tracking() {
    let db = create_test_db();
    let cache = LmdbCacheReaderWriter::new(db);

    // Put some values
    cache.put("key1", b"value1").unwrap();
    cache.put("key2", b"value2").unwrap();
    cache.read("key3").unwrap(); // This should also track the key

    // Complete session should process all tracked keys
    cache.complete_session().unwrap();

    // Verify metadata was stored
    let metadata_result = cache.read("memo_cache_metadata").unwrap();
    assert!(metadata_result.is_some());

    let metadata: CacheTracker = bincode::deserialize(&metadata_result.unwrap()).unwrap();
    assert_eq!(metadata.entries.len(), 3);
    assert!(metadata.key_to_timestamp.contains_key("key1"));
    assert!(metadata.key_to_timestamp.contains_key("key2"));
    assert!(metadata.key_to_timestamp.contains_key("key3"));
  }

  #[test]
  fn test_max_entries_enforcement() {
    let db = create_test_db();
    let cache = LmdbCacheReaderWriter::with_max_entries(db, 3); // Set low limit for testing

    // Add more entries than max_entries
    for i in 0..5 {
      cache
        .put(&format!("key{}", i), &format!("value{}", i).into_bytes())
        .unwrap();
      // Sleep a bit to ensure different timestamps
      std::thread::sleep(std::time::Duration::from_millis(1));
    }

    cache.complete_session().unwrap();

    // Check that only max_entries remain
    let metadata_result = cache.read("memo_cache_metadata").unwrap();
    let metadata: CacheTracker = bincode::deserialize(&metadata_result.unwrap()).unwrap();
    assert_eq!(metadata.entries.len(), 3);
  }

  #[test]
  fn test_multiple_sessions() {
    let db = create_test_db();
    let cache = LmdbCacheReaderWriter::new(db);

    // First session
    cache.put("session1_key", b"value1").unwrap();
    cache.complete_session().unwrap();

    // Second session
    cache.put("session2_key", b"value2").unwrap();
    cache.complete_session().unwrap();

    // Verify both keys are tracked
    let metadata_result = cache.read("memo_cache_metadata").unwrap();
    let metadata: CacheTracker = bincode::deserialize(&metadata_result.unwrap()).unwrap();
    assert_eq!(metadata.entries.len(), 2);
    assert!(metadata.key_to_timestamp.contains_key("session1_key"));
    assert!(metadata.key_to_timestamp.contains_key("session2_key"));
  }

  #[test]
  fn test_cache_entry_ordering() {
    let entry1 = CacheEntry {
      timestamp: 1000,
      key: "key1".to_string(),
    };
    let entry2 = CacheEntry {
      timestamp: 2000,
      key: "key2".to_string(),
    };
    let entry3 = CacheEntry {
      timestamp: 1000,
      key: "key2".to_string(),
    }; // Same timestamp, different key

    assert!(entry1 < entry2); // Earlier timestamp comes first
    assert!(entry1 < entry3); // Same timestamp, lexicographic key order
    assert!(entry3 < entry2); // Earlier timestamp comes first
  }

  #[test]
  fn test_empty_session() {
    let db = create_test_db();
    let cache = LmdbCacheReaderWriter::new(db);

    // Complete session without any operations
    cache.complete_session().unwrap();

    // Should create empty metadata
    let metadata_result = cache.read("memo_cache_metadata").unwrap();
    let metadata: CacheTracker = bincode::deserialize(&metadata_result.unwrap()).unwrap();
    assert_eq!(metadata.entries.len(), 0);
  }
}
