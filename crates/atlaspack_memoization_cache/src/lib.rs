use lmdb_js_lite::writer::DatabaseWriter;
use rand::Rng;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

pub trait CacheReaderWriter {
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;
}

pub struct LmdbCacheReaderWriter {
  db_writer: Arc<DatabaseWriter>,
}

impl LmdbCacheReaderWriter {
  pub fn new(db_writer: Arc<DatabaseWriter>) -> Self {
    Self { db_writer }
  }
}

impl CacheReaderWriter for LmdbCacheReaderWriter {
  #[tracing::instrument(level = "trace", skip_all)]
  fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let transaction = self.db_writer.read_txn()?;

    self
      .db_writer
      .get(&transaction, key)
      .map_err(|error| anyhow::anyhow!("Read transaction error: {}", error))
  }

  #[tracing::instrument(level = "trace", skip_all)]
  fn put(&self, key: &str, value: &[u8]) -> anyhow::Result<()> {
    let mut transaction = self.db_writer.write_txn()?;

    self.db_writer.put(&mut transaction, key, value)?;

    transaction
      .commit()
      .map_err(|error| anyhow::anyhow!("Put commit error: {}", error))
  }
}

pub struct CacheHandler<T: CacheReaderWriter> {
  reader_writer: T,
  validation_rate: f32, // Probability of running validation (0.0 to 1.0)
}

impl<T: CacheReaderWriter> CacheHandler<T> {
  pub fn new(reader_writer: T) -> Self {
    CacheHandler {
      reader_writer,
      validation_rate: 0.0, // No validation by default
    }
  }

  pub fn new_with_validation(reader_writer: T, validation_rate: f32) -> Self {
    let validation_rate = validation_rate.clamp(0.0, 1.0);
    CacheHandler {
      reader_writer,
      validation_rate,
    }
  }

  #[tracing::instrument(level = "debug", skip_all, fields(label))]
  pub async fn run<Input, RunFn, Res, FutureResult, Error>(
    &self,
    label: &str,
    input: Input,
    run_fn: RunFn,
  ) -> Result<Res, Error>
  where
    Input: Hash,
    Res: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug,
    FutureResult: Future<Output = Result<Res, Error>>,
    RunFn: FnOnce(Input) -> FutureResult,
  {
    // First hash the label and input to create a cache key
    let cache_key = hash_input(&input, label);

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
          return Ok(value);
        }
        Err(err) => {
          tracing::error!("Failed to deserialize cached value for {}: {}", label, err);
        }
      }
    }

    // Otherwise, run the function to get the result
    let result = run_fn(input).await?;

    // Now we have a result, try to serialize and cache it
    match serialize(&result) {
      Ok(serialized_result) => {
        if let Some(value) = cache_result
          && value != serialized_result
        {
          tracing::error!(
            "Cache validation mismatch for {}: cached value does not match computed result",
            label
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

#[tracing::instrument(level = "trace", skip_all, fields(label), ret)]
fn hash_input<Input: Hash>(input: &Input, label: &str) -> String {
  let mut hasher = DefaultHasher::new();
  input.hash(&mut hasher);
  format!("{}-{}", label, hasher.finish())
}

#[cfg(test)]
mod test {
  use parking_lot::Mutex;
  use std::collections::HashMap;

  use super::*;
  use pretty_assertions::assert_eq;
  use serde::{Deserialize, Serialize};

  #[tokio::test]
  async fn handles_no_cached_value() {
    let reader_writer = SimpleMockReaderWriter::new_empty();

    let handler = CacheHandler::new(reader_writer);

    let result = handler
      .run("hello_result", "Hello", |input| async move {
        Ok::<_, ()>(input.to_ascii_uppercase())
      })
      .await;

    assert_eq!(result, Ok("HELLO".to_string()));
  }

  #[tokio::test]
  async fn handles_cached_value() {
    #[derive(PartialEq, Debug, Deserialize, Serialize)]
    enum TestResult {
      Success,
      Fail,
    }

    let cached_value = serde_json::to_vec(&TestResult::Success).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    let handler = CacheHandler::new(reader_writer);

    let result = handler
      .run("test_label", "Hello", |_input| async move {
        Ok::<_, ()>(TestResult::Fail)
      })
      .await;

    assert_eq!(result, Ok(TestResult::Success));
  }

  #[tokio::test]
  async fn caches_second_call() {
    let reader_writer = InMemoryReaderWriter::default();
    let handler = CacheHandler::new(reader_writer);

    let result = handler
      .run("test_label", "Hello", |input| async move {
        Ok::<_, ()>(input.to_ascii_uppercase())
      })
      .await;
    assert_eq!(result, Ok("HELLO".to_string()));

    let result: Result<String, ()> = handler
      .run("test_label", "Hello", |_input| async move {
        panic!("Should not call 2nd run function")
      })
      .await;
    assert_eq!(result, Ok("HELLO".to_string()));
  }

  #[tokio::test]
  async fn handles_cache_read_error() {
    let reader_writer = ErrorMockReaderWriter::new_with_read_error();
    let handler = CacheHandler::new(reader_writer);

    // Should still succeed even if cache read fails
    let result = handler
      .run("test_label", "Hello", |input| async move {
        Ok::<_, ()>(input.to_ascii_uppercase())
      })
      .await;
    assert_eq!(result, Ok("HELLO".to_string()));
  }

  #[tokio::test]
  async fn handles_cache_write_error() {
    let reader_writer = ErrorMockReaderWriter::new_with_write_error();
    let handler = CacheHandler::new(reader_writer);

    // Should still succeed even if cache write fails
    let result = handler
      .run("test_label", "Hello", |input| async move {
        Ok::<_, ()>(input.to_ascii_uppercase())
      })
      .await;
    assert_eq!(result, Ok("HELLO".to_string()));
  }

  #[tokio::test]
  async fn cache_validation_with_matching_results() {
    #[derive(PartialEq, Debug, Deserialize, Serialize)]
    struct TestResult {
      value: String,
    }

    let cached_result = TestResult {
      value: "HELLO".to_string(),
    };
    let cached_value = serde_json::to_vec(&cached_result).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    // Set validation rate to 100% to ensure validation runs
    let handler = CacheHandler::new_with_validation(reader_writer, 1.0);

    let result: Result<TestResult, ()> = handler
      .run("test_label", "Hello", |input| async move {
        Ok::<_, ()>(TestResult {
          value: input.to_ascii_uppercase(),
        })
      })
      .await;

    // Should return cached result even with validation
    assert_eq!(
      result,
      Ok(TestResult {
        value: "HELLO".to_string()
      })
    );
  }

  #[tokio::test]
  async fn cache_validation_with_mismatched_results() {
    #[derive(PartialEq, Debug, Deserialize, Serialize)]
    struct TestResult {
      value: String,
    }

    // Cache contains "WRONG" but computation would return "HELLO"
    let cached_result = TestResult {
      value: "WRONG".to_string(),
    };
    let cached_value = serde_json::to_vec(&cached_result).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    // Set validation rate to 100% to ensure validation runs
    let handler = CacheHandler::new_with_validation(reader_writer, 1.0);

    let result: Result<TestResult, ()> = handler
      .run("test_label", "Hello", |input| async move {
        Ok::<_, ()>(TestResult {
          value: input.to_ascii_uppercase(),
        })
      })
      .await;

    // NOTE: Current implementation returns computed result when validation is enabled
    // This means validation always "fixes" invalid cache entries by returning fresh computation
    assert_eq!(
      result,
      Ok(TestResult {
        value: "HELLO".to_string()
      })
    );
  }

  #[tokio::test]
  async fn cache_validation_disabled_by_default() {
    #[derive(PartialEq, Debug, Deserialize, Serialize)]
    struct TestResult {
      value: String,
    }

    let cached_result = TestResult {
      value: "CACHED".to_string(),
    };
    let cached_value = serde_json::to_vec(&cached_result).unwrap();
    let reader_writer = SimpleMockReaderWriter::new_with_cached_value(cached_value);

    // Default handler has validation disabled
    let handler = CacheHandler::new(reader_writer);

    let result: Result<TestResult, ()> = handler
      .run("test_label", "Hello", |_input| async move {
        panic!("Validation should not run - this would execute if validation was enabled")
      })
      .await;

    assert_eq!(
      result,
      Ok(TestResult {
        value: "CACHED".to_string()
      })
    );
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
