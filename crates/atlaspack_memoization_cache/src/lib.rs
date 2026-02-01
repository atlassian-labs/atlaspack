use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

mod lmdb_cache_reader_writer;

pub use lmdb_cache_reader_writer::LmdbCacheReaderWriter;

pub trait CacheReaderWriter: Send + Sync {
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;
  fn complete_session(&self) -> anyhow::Result<()> {
    Ok(())
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
  bailouts: AtomicU64,
  errors: AtomicU64,
  validations: AtomicU64,
}

impl Default for Stats {
  fn default() -> Self {
    Self {
      hits: AtomicU64::new(0),
      misses: AtomicU64::new(0),
      uncacheables: AtomicU64::new(0),
      bailouts: AtomicU64::new(0),
      errors: AtomicU64::new(0),
      validations: AtomicU64::new(0),
    }
  }
}

impl Stats {
  fn clear(&self) {
    self.hits.store(0, Ordering::Relaxed);
    self.misses.store(0, Ordering::Relaxed);
    self.uncacheables.store(0, Ordering::Relaxed);
    self.bailouts.store(0, Ordering::Relaxed);
    self.errors.store(0, Ordering::Relaxed);
    self.validations.store(0, Ordering::Relaxed);
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

  fn increment_bailouts(&self) {
    self.bailouts.fetch_add(1, Ordering::Relaxed);
  }

  fn increment_errors(&self) {
    self.errors.fetch_add(1, Ordering::Relaxed);
  }

  fn increment_validations(&self) {
    self.validations.fetch_add(1, Ordering::Relaxed);
  }

  fn get_snapshot(&self) -> StatsSnapshot {
    StatsSnapshot {
      hits: self.hits.load(Ordering::Relaxed),
      misses: self.misses.load(Ordering::Relaxed),
      uncacheables: self.uncacheables.load(Ordering::Relaxed),
      bailouts: self.bailouts.load(Ordering::Relaxed),
      errors: self.errors.load(Ordering::Relaxed),
      validations: self.validations.load(Ordering::Relaxed),
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSnapshot {
  pub hits: u64,
  pub misses: u64,
  pub uncacheables: u64,
  pub bailouts: u64,
  pub errors: u64,
  pub validations: u64,
}

pub enum CacheMode {
  Off,
  /// Validate a percentage of runs, but never return cached values
  Shadow(f32),
  /// Enable with a percentage of validation runs
  On(f32),
}

pub struct CacheHandler<T: CacheReaderWriter> {
  reader_writer: T,
  stats: Stats,
  mode: CacheMode,
}

#[async_trait]
pub trait CacheHandlerTrait {
  /// Get a snapshot of the current cache statistics without clearing them.
  /// Useful for intermediate logging during a build.
  fn get_stats_snapshot(&self) -> StatsSnapshot;

  fn complete_session(&self) -> anyhow::Result<StatsSnapshot>;

  async fn run<Input, RunFn, Res, FutureResult, Error>(
    &self,
    input: Input,
    run_fn: RunFn,
  ) -> Result<Res, Error>
  where
    Input: Cacheable + Send,
    Res: CacheResponse + Send,
    Error: Send,
    FutureResult: Future<Output = Result<Res, Error>> + Send,
    RunFn: FnOnce(Input) -> FutureResult + Send;
}

impl<T: CacheReaderWriter> CacheHandler<T> {
  pub fn new(reader_writer: T, mode: CacheMode) -> Self {
    CacheHandler {
      reader_writer,
      stats: Stats::default(),
      mode,
    }
  }

  async fn run_with_cache<Input, RunFn, Res, FutureResult, Error>(
    &self,
    label: String,
    cache_key: String,
    input: Input,
    run_fn: RunFn,
    should_validate: bool,
  ) -> Result<Res, Error>
  where
    Input: Cacheable,
    Res: CacheResponse,
    FutureResult: Future<Output = Result<Res, Error>>,
    RunFn: FnOnce(Input) -> FutureResult,
  {
    // Then check the cache for an existing value
    let cache_result = match self.reader_writer.read(&cache_key) {
      Ok(value) => value,
      Err(err) => {
        // We don't want to blow up the build for a cache read error, so we log it and
        // continue
        tracing::error!("Failed to read cache entry for {}:\n{}", label, err);
        self.stats.increment_errors();
        None
      }
    };

    if !should_validate && let Some(value) = cache_result.as_ref() {
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
          self.stats.increment_errors();
        }
      }
    }

    // Otherwise, run the function to get the result (cache miss)
    let computed_result = run_fn(input).await?;

    if computed_result.should_bailout() {
      // If the result indicates we should bailout, do not cache it
      tracing::debug!("Bailing out of caching for {}", label);
      self.stats.increment_bailouts();
      return Ok(computed_result);
    }

    // Now we have a result, try to serialize and cache it
    match serialize(&computed_result) {
      Ok(serialized_result) => {
        if let Some(cached_result) = cache_result
          && let Ok(cached_result) = deserialize::<Res>(&cached_result)
        {
          self.stats.increment_validations();

          if cached_result != computed_result {
            let comparison = pretty_assertions::Comparison::new(&cached_result, &computed_result);

            tracing::error!(
              "Cache validation mismatch for {}: cached value does not match computed result\n{}",
              label,
              comparison
            );
          }
        } else {
          self.stats.increment_misses();

          if let Err(error) = self.reader_writer.put(&cache_key, &serialized_result) {
            tracing::error!("Failed to write to cache for {}: {}", label, error);
            self.stats.increment_errors();
          }
        }
      }
      Err(err) => {
        tracing::error!("Failed to serialize result for {}: {}", label, err);
        self.stats.increment_errors();
      }
    }

    // Finally return the result
    Ok(computed_result)
  }

  /// Determines if we should validate this cache hit based on the configured sampling rate
  fn should_validate(&self, label: &str, validation_rate: f32) -> bool {
    // Use label as seed to get consistent results per key
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let hash = hasher.finish();

    // Convert hash to a float between 0.0 and 1.0
    let normalized = (hash as f64) / (u64::MAX as f64);

    normalized < validation_rate as f64
  }
}

#[async_trait]
impl<T: CacheReaderWriter> CacheHandlerTrait for CacheHandler<T> {
  fn get_stats_snapshot(&self) -> StatsSnapshot {
    self.stats.get_snapshot()
  }

  fn complete_session(&self) -> anyhow::Result<StatsSnapshot> {
    let snapshot = self.stats.get_snapshot();
    tracing::info!("Cache stats {:#?}", snapshot);

    self.stats.clear();
    self.reader_writer.complete_session()?;

    Ok(snapshot)
  }

  #[tracing::instrument(level = "debug", skip_all)]
  async fn run<Input, RunFn, Res, FutureResult, Error>(
    &self,
    input: Input,
    run_fn: RunFn,
  ) -> Result<Res, Error>
  where
    Input: Cacheable + Send,
    Res: CacheResponse + Send,
    Error: Send,
    FutureResult: Future<Output = Result<Res, Error>> + Send,
    RunFn: FnOnce(Input) -> FutureResult + Send,
  {
    match self.mode {
      CacheMode::Off => run_fn(input).await,
      CacheMode::Shadow(validation_rate) => {
        // First hash the label and input to create a cache key
        let Some((label, cache_key)) = input.cache_key() else {
          // Input isn't cacheable so just run the function directly
          self.stats.increment_uncacheables();
          return run_fn(input).await;
        };

        if !self.should_validate(&label, validation_rate) {
          return run_fn(input).await;
        }

        self
          .run_with_cache(label, cache_key, input, run_fn, true)
          .await
      }
      CacheMode::On(validation_rate) => {
        // First hash the label and input to create a cache key
        let Some((label, cache_key)) = input.cache_key() else {
          // Input isn't cacheable so just run the function directly
          self.stats.increment_uncacheables();
          return run_fn(input).await;
        };

        let should_validate = self.should_validate(&label, validation_rate);

        self
          .run_with_cache(label, cache_key, input, run_fn, should_validate)
          .await
      }
    }
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
  fn should_bailout(&self) -> bool;
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
    fn should_bailout(&self) -> bool {
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

    let handler = CacheHandler::new(reader_writer, CacheMode::On(0.0));

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

    let handler = CacheHandler::new(reader_writer, CacheMode::On(0.0));

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
    let handler = CacheHandler::new(reader_writer, CacheMode::On(0.0));

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
    let handler = CacheHandler::new(reader_writer, CacheMode::On(0.0));

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
    let handler = CacheHandler::new(reader_writer, CacheMode::On(0.0));

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
    let handler = CacheHandler::new(reader_writer, CacheMode::On(1.0));

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
    let handler = CacheHandler::new(reader_writer, CacheMode::On(1.0));

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
    let handler = CacheHandler::new(reader_writer, CacheMode::On(0.0));

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
