use lmdb_js_lite::DatabaseHandle;
use rand::Rng;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub trait CacheReaderWriter {
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;
}

pub struct LmdbCacheReaderWriter {
  db_handle: Arc<DatabaseHandle>,
}

impl LmdbCacheReaderWriter {
  pub fn new(db_handle: Arc<DatabaseHandle>) -> Self {
    Self { db_handle }
  }
}

impl CacheReaderWriter for LmdbCacheReaderWriter {
  #[tracing::instrument(level = "trace", skip_all)]
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let db_writer = self.db_handle.database();
    let transaction = db_writer.read_txn()?;

    db_writer
      .get(&transaction, key)
      .map_err(|error| anyhow::anyhow!("Read transaction error: {}", error))
  }

  #[tracing::instrument(level = "trace", skip_all)]
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    let db_writer = self.db_handle.database();
    let mut transaction = db_writer.write_txn()?;

    db_writer.put(&mut transaction, key, value)?;

    transaction
      .commit()
      .map_err(|error| anyhow::anyhow!("Put commit error: {}", error))
  }
}

/// In-memory cache implementation for testing
#[derive(Default)]
pub struct InMemoryReaderWriter {
  store: parking_lot::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl CacheReaderWriter for InMemoryReaderWriter {
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    Ok(self.store.lock().get(key).cloned())
  }

  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    self
      .store
      .lock()
      .insert(String::from(key), value.to_owned());
    Ok(())
  }
}

#[derive(Debug)]
struct Stats {
  hits: AtomicU64,
  misses: AtomicU64,
  uncacheables: AtomicU64,
}

impl Default for Stats {
  fn default() -> Self {
    Self {
      hits: AtomicU64::new(0),
      misses: AtomicU64::new(0),
      uncacheables: AtomicU64::new(0),
    }
  }
}

impl Stats {
  fn clear(&self) {
    self.hits.store(0, Ordering::Relaxed);
    self.misses.store(0, Ordering::Relaxed);
    self.uncacheables.store(0, Ordering::Relaxed);
  }

  fn increment_hits(&self) {
    self.hits.fetch_add(1, Ordering::Relaxed);
  }

  fn increment_misses(&self) {
    self.misses.fetch_add(1, Ordering::Relaxed);
  }

  fn increment_uncacheables(&self) {
    self.uncacheables.fetch_add(1, Ordering::Relaxed);
  }

  fn get_snapshot(&self) -> StatsSnapshot {
    StatsSnapshot {
      hits: self.hits.load(Ordering::Relaxed),
      misses: self.misses.load(Ordering::Relaxed),
      uncacheables: self.uncacheables.load(Ordering::Relaxed),
    }
  }
}

#[derive(Debug, Clone)]
pub struct StatsSnapshot {
  pub hits: u64,
  pub misses: u64,
  pub uncacheables: u64,
}

pub struct CacheHandler<T: CacheReaderWriter> {
  reader_writer: T,
  validation_rate: f32, // Probability of running validation (0.0 to 1.0)
  stats: Stats,
}

impl<T: CacheReaderWriter> CacheHandler<T> {
  pub fn new(reader_writer: T) -> Self {
    CacheHandler {
      reader_writer,
      validation_rate: 0.0, // No validation by default
      stats: Stats::default(),
    }
  }

  pub fn new_with_validation(reader_writer: T, validation_rate: f32) -> Self {
    let validation_rate = validation_rate.clamp(0.0, 1.0);
    CacheHandler {
      reader_writer,
      validation_rate,
      stats: Stats::default(),
    }
  }

  pub fn reset_stats(&self) {
    let snapshot = self.stats.get_snapshot();
    tracing::info!("Cache stats {:#?}", snapshot);
    self.stats.clear();
  }

  pub fn get_stats(&self) -> StatsSnapshot {
    self.stats.get_snapshot()
  }

  #[tracing::instrument(level = "debug", skip_all)]
  pub async fn run<Input, RunFn, Res, FutureResult, Error>(
    &self,
    input: Input,
    run_fn: RunFn,
  ) -> Result<Res, Error>
  where
    Input: Cacheable,
    Res: CacheResponse,
    FutureResult: Future<Output = Result<Res, Error>>,
    RunFn: FnOnce(Input) -> FutureResult,
  {
    // First hash the label and input to create a cache key
    let Some((label, cache_key)) = input.cache_key() else {
      // Input isn't cacheable so just run the function directly
      self.stats.increment_uncacheables();
      return run_fn(input).await;
    };

    // Then check the cache for an existing value
    let cache_result = match self.reader_writer.read(&cache_key) {
      Ok(value) => value,
      Err(err) => {
        // We don't want to blow up the build for a cache read error, so we log it and
        // continue
        tracing::error!("Failed to read cache entry for {}:\n{}", label, err);
        None
      }
    };

    // If we have a cached value, try to deserialize it and return it
    if !self.should_validate()
      && let Some(value) = cache_result.as_ref()
    {
      match deserialize(value) {
        Ok(value) => {
          self.stats.increment_hits();
          return Ok(value);
        }
        Err(err) => {
          // Log the first 200 chars of the serialized data to help debug
          let preview = String::from_utf8_lossy(&value[..value.len().min(200)]);
          tracing::error!(
            "Failed to deserialize cached value for {}: {}\nSerialized data preview (first 200 chars): {}",
            label,
            err,
            preview
          );
        }
      }
    }

    // Otherwise, run the function to get the result (cache miss)
    self.stats.increment_misses();
    let result = run_fn(input).await?;

    if result.bailout() {
      // If the result indicates we should bailout, do not cache it
      tracing::debug!("Bailing out of caching for {}", label);
      return Ok(result);
    }

    // Now we have a result, try to serialize and cache it
    match serialize(&result) {
      Ok(serialized_result) => {
        if let Some(value) = cache_result
          && let Ok(cached) = deserialize::<Res>(&value)
          && cached != result
        {
          let comparison = pretty_assertions::Comparison::new(&cached, &result);
          tracing::error!(
            "Cache validation mismatch for {}: cached value does not match computed result\n{}",
            label,
            comparison
          );
        }

        if let Err(error) = self.reader_writer.put(&cache_key, &serialized_result) {
          tracing::error!("Failed to write to cache for {}: {}", label, error);
        }
      }
      Err(err) => {
        tracing::error!("Failed to serialize result for {}: {}", label, err);
      }
    }

    // Finally return the result
    Ok(result)
  }

  /// Determines if we should validate this cache hit based on the configured sampling rate
  fn should_validate(&self) -> bool {
    let random: f32 = rand::thread_rng().gen_range(0.0..1.0);
    random < self.validation_rate
  }
}

#[tracing::instrument(level = "trace", skip_all)]
fn deserialize<Output: DeserializeOwned>(value: &[u8]) -> Result<Output, serde_json::Error> {
  serde_json::from_slice(value)
}

#[tracing::instrument(level = "trace", skip_all)]
fn serialize<Input: Serialize>(value: Input) -> Result<Vec<u8>, serde_json::Error> {
  serde_json::to_vec(&value)
}

pub trait Cacheable {
  fn cache_key(&self) -> Option<(String, String)>;
}

pub trait CacheResponse: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug {
  fn bailout(&self) -> bool;
}

#[cfg(test)]
mod test {
  use parking_lot::Mutex;
  use std::collections::HashMap;

  use super::*;
  use pretty_assertions::assert_eq;
  use serde::{Deserialize, Serialize};

  // Test response type that implements CacheResponse
  #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
  struct TestResponse {
    value: String,
  }

  impl TestResponse {
    fn new(value: &str) -> Self {
      Self {
        value: value.to_string(),
      }
    }
  }

  impl CacheResponse for TestResponse {
    fn bailout(&self) -> bool {
      false
    }
  }

  // Simple cacheable wrapper for testing
  #[derive(Debug)]
  struct CacheableString {
    label: String,
    value: String,
  }

  impl CacheableString {
    fn new(label: &str, value: &str) -> Self {
      Self {
        label: label.to_string(),
        value: value.to_string(),
      }
    }
  }

  impl Cacheable for CacheableString {
    fn cache_key(&self) -> Option<(String, String)> {
      Some((self.label.clone(), format!("{}:{}", self.label, self.value)))
    }
  }

  #[tokio::test]
  async fn handles_no_cached_value() {
    let reader_writer = SimpleMockReaderWriter::new_empty();

    let handler = CacheHandler::new(reader_writer);

    let input = CacheableString::new("hello_result", "Hello");
    let result = handler
      .run(input, |input| async move {
        Ok::<_, ()>(TestResponse::new(&input.value.to_ascii_uppercase()))
      })
      .await;

    assert_eq!(result, Ok(TestResponse::new("HELLO")));
  }

  #[tokio::test]
  async fn handles_cached_value() {
    let cached_response = TestResponse::new("CACHED");
    let cached_value = serde_json::to_vec(&cached_response).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    let handler = CacheHandler::new(reader_writer);

    let input = CacheableString::new("test_label", "Hello");
    let result = handler
      .run(input, |_input| async move {
        Ok::<_, ()>(TestResponse::new("COMPUTED"))
      })
      .await;

    // Should return cached value, not computed
    assert_eq!(result, Ok(TestResponse::new("CACHED")));
  }

  #[tokio::test]
  async fn caches_second_call() {
    let reader_writer = InMemoryReaderWriter::default();
    let handler = CacheHandler::new(reader_writer);

    let input1 = CacheableString::new("test_label", "Hello");
    let result = handler
      .run(input1, |input| async move {
        Ok::<_, ()>(TestResponse::new(&input.value.to_ascii_uppercase()))
      })
      .await;
    assert_eq!(result, Ok(TestResponse::new("HELLO")));

    let input2 = CacheableString::new("test_label", "Hello");
    let result: Result<TestResponse, ()> = handler
      .run(input2, |_input| async move {
        panic!("Should not call 2nd run function")
      })
      .await;
    assert_eq!(result, Ok(TestResponse::new("HELLO")));
  }

  #[tokio::test]
  async fn handles_cache_read_error() {
    let reader_writer = ErrorMockReaderWriter::new_with_read_error();
    let handler = CacheHandler::new(reader_writer);

    // Should still succeed even if cache read fails
    let input = CacheableString::new("test_label", "Hello");
    let result = handler
      .run(input, |input| async move {
        Ok::<_, ()>(TestResponse::new(&input.value.to_ascii_uppercase()))
      })
      .await;
    assert_eq!(result, Ok(TestResponse::new("HELLO")));
  }

  #[tokio::test]
  async fn handles_cache_write_error() {
    let reader_writer = ErrorMockReaderWriter::new_with_write_error();
    let handler = CacheHandler::new(reader_writer);

    // Should still succeed even if cache write fails
    let input = CacheableString::new("test_label", "Hello");
    let result = handler
      .run(input, |input| async move {
        Ok::<_, ()>(TestResponse::new(&input.value.to_ascii_uppercase()))
      })
      .await;
    assert_eq!(result, Ok(TestResponse::new("HELLO")));
  }

  #[tokio::test]
  async fn cache_validation_with_matching_results() {
    let cached_response = TestResponse::new("HELLO");
    let cached_value = serde_json::to_vec(&cached_response).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    // Set validation rate to 100% to ensure validation runs
    let handler = CacheHandler::new_with_validation(reader_writer, 1.0);

    let input = CacheableString::new("test_label", "Hello");
    let result: Result<TestResponse, ()> = handler
      .run(input, |input| async move {
        Ok::<_, ()>(TestResponse::new(&input.value.to_ascii_uppercase()))
      })
      .await;

    // Should return computed result when validation is enabled
    assert_eq!(result, Ok(TestResponse::new("HELLO")));
  }

  #[tokio::test]
  async fn cache_validation_with_mismatched_results() {
    // Cache contains "WRONG" but computation would return "HELLO"
    let cached_response = TestResponse::new("WRONG");
    let cached_value = serde_json::to_vec(&cached_response).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    // Set validation rate to 100% to ensure validation runs
    let handler = CacheHandler::new_with_validation(reader_writer, 1.0);

    let input = CacheableString::new("test_label", "Hello");
    let result: Result<TestResponse, ()> = handler
      .run(input, |input| async move {
        Ok::<_, ()>(TestResponse::new(&input.value.to_ascii_uppercase()))
      })
      .await;

    // NOTE: Current implementation returns computed result when validation is enabled
    // This means validation always "fixes" invalid cache entries by returning fresh computation
    assert_eq!(result, Ok(TestResponse::new("HELLO")));
  }

  #[tokio::test]
  async fn cache_validation_disabled_by_default() {
    let cached_response = TestResponse::new("CACHED");
    let cached_value = serde_json::to_vec(&cached_response).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    // Default handler has validation disabled
    let handler = CacheHandler::new(reader_writer);

    let input = CacheableString::new("test_label", "Hello");
    let result: Result<TestResponse, ()> = handler
      .run(input, |_input| async move {
        panic!("Validation should not run - this would execute if validation was enabled")
      })
      .await;

    assert_eq!(result, Ok(TestResponse::new("CACHED")));
  }

  // Simple mock for testing cache miss scenarios
  struct SimpleMockReaderWriter {
    cached_value: Option<Vec<u8>>,
  }

  impl SimpleMockReaderWriter {
    fn new_empty() -> Self {
      Self { cached_value: None }
    }

    fn new_with_cached_value(value: Vec<u8>) -> Self {
      Self {
        cached_value: Some(value),
      }
    }
  }

  impl CacheReaderWriter for SimpleMockReaderWriter {
    fn read(&self, _key: &str) -> anyhow::Result<Option<Vec<u8>>> {
      Ok(self.cached_value.clone())
    }

    fn put(&self, _key: &str, _value: &[u8]) -> anyhow::Result<()> {
      Ok(())
    }
  }

  // In-memory cache implementation for testing actual caching behavior
  #[derive(Default)]
  struct InMemoryReaderWriter {
    store: Mutex<HashMap<String, Vec<u8>>>,
  }

  impl CacheReaderWriter for InMemoryReaderWriter {
    fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
      Ok(self.store.lock().get(key).cloned())
    }

    fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
      self
        .store
        .lock()
        .insert(String::from(key), value.to_owned());
      Ok(())
    }
  }

  // Mock for testing error scenarios
  struct ErrorMockReaderWriter {
    read_error: bool,
    write_error: bool,
  }

  impl ErrorMockReaderWriter {
    fn new_with_read_error() -> Self {
      Self {
        read_error: true,
        write_error: false,
      }
    }

    fn new_with_write_error() -> Self {
      Self {
        read_error: false,
        write_error: true,
      }
    }
  }

  impl CacheReaderWriter for ErrorMockReaderWriter {
    fn read(&self, _key: &str) -> anyhow::Result<Option<Vec<u8>>> {
      if self.read_error {
        Err(anyhow::anyhow!("Simulated read error"))
      } else {
        Ok(None)
      }
    }

    fn put(&self, _key: &str, _value: &[u8]) -> anyhow::Result<()> {
      if self.write_error {
        Err(anyhow::anyhow!("Simulated write error"))
      } else {
        Ok(())
      }
    }
  }
}
