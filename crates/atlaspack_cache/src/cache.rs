use lmdb_js_lite::writer::DatabaseWriter;
use mockall::automock;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

#[automock]
pub trait CacheReaderWriter {
  fn read(&self, key: &str) -> Option<Vec<u8>>;
  // TODO: error handling
  fn put(&self, key: &str, value: &[u8]) -> Result<(), ()>;
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
  #[tracing::instrument(level = "debug", skip_all)]
  fn read(&self, key: &str) -> Option<Vec<u8>> {
    let transaction = self
      .db_writer
      .read_txn()
      .map_err(|_| {
        tracing::error!("[Error] Create read transaction");
      })
      .ok()?;
    self
      .db_writer
      .get(&transaction, key)
      .map_err(|_| tracing::error!("[Error] Get error"))
      .ok()?
  }

  #[tracing::instrument(level = "debug", skip_all)]
  fn put(&self, key: &str, value: &[u8]) -> Result<(), ()> {
    let mut transaction = self.db_writer.environment().write_txn().map_err(|_| {
      tracing::error!("[Error] Create write transaction");
    })?;
    self
      .db_writer
      .put(&mut transaction, key, value)
      .map_err(|_| tracing::error!("[Error] Put error"))?;
    transaction
      .commit()
      .map_err(|_| tracing::error!("[Error] Put transaction error"))
  }
}

pub struct CacheHandler<T: CacheReaderWriter> {
  reader_writer: T,
}

impl<T: CacheReaderWriter> CacheHandler<T> {
  pub fn new(reader_writer: T) -> Self {
    CacheHandler { reader_writer }
  }

  #[tracing::instrument(level = "info", skip_all, fields(label))]
  pub async fn run<Input, RunFn, Res, FutureResult, Error>(
    &self,
    label: &str,
    input: Input,
    run_fn: RunFn,
  ) -> Result<Res, Error>
  where
    Input: Hash,
    Res: DeserializeOwned + Serialize,
    FutureResult: Future<Output = Result<Res, Error>>,
    RunFn: FnOnce(Input) -> FutureResult,
  {
    let input_hash = hash_input(&input, label);

    if let Some(value) = self.reader_writer.read(&input_hash) {
      let value: Res = deserialize(&value);

      return Ok(value);
    }

    let result = run_fn(input).await?;

    let serialized_result = serialize(&result);

    self
      .reader_writer
      .put(&input_hash, &serialized_result)
      .expect("Ohh oh 2");

    Ok(result)
  }
}

#[tracing::instrument(level = "debug", skip_all)]
fn serialize<Input: Serialize>(value: Input) -> Vec<u8> {
  serde_json::to_vec(&value).expect("Ohh oh")
}

#[tracing::instrument(level = "info", skip_all)]
fn deserialize<Output: DeserializeOwned>(value: &[u8]) -> Output {
  serde_json::from_slice(value).expect("Deserialize fail")
}

#[tracing::instrument(level = "debug", skip_all, fields(label), ret)]
fn hash_input<Input: Hash>(input: &Input, label: &str) -> String {
  let mut hasher = DefaultHasher::new();
  input.hash(&mut hasher);
  format!("{}-{}", label, hasher.finish())
}

#[cfg(test)]
mod test {
  use std::collections::HashMap;

  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn handles_no_cached_value() {
    let mut reader_writer = MockCacheReaderWriter::default();
    reader_writer.expect_read().return_once(|_| None);
    reader_writer.expect_put().return_once(|_, _| Ok(()));

    let mut handler = CacheHandler::new(reader_writer);

    let result = handler.run("Hello", |input| input.to_ascii_uppercase());

    assert_eq!(result, "HELLO");
  }

  #[test]
  fn handles_cached_value() {
    #[derive(PartialEq, Debug, Deserialize, Serialize)]
    enum Result {
      Success,
      Fail,
    }
    let mut reader_writer = MockCacheReaderWriter::default();
    reader_writer
      .expect_read()
      .return_once(|_| Some(serde_json::to_vec(&Result::Success).unwrap()));

    let mut handler = CacheHandler::new(reader_writer);

    let result = handler.run("Hello", |_input| Result::Fail);

    assert_eq!(result, Result::Success);
  }

  #[test]
  fn caches_second_call() {
    let reader_writer = TestReaderWriter::default();
    let mut handler = CacheHandler::new(reader_writer);

    let result = handler.run("Hello", |input| input.to_ascii_uppercase());
    assert_eq!(result, "HELLO");

    let result: String = handler.run("Hello", |_input| panic!("Should not call 2nd run function"));
    assert_eq!(result, "HELLO");
  }

  #[derive(Default)]
  struct TestReaderWriter {
    store: HashMap<String, Vec<u8>>,
  }

  impl CacheReaderWriter for TestReaderWriter {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
      self.store.get(key).cloned()
    }

    fn put(&mut self, key: &str, value: &[u8]) -> Result<(), ()> {
      self.store.insert(String::from(key), value.to_owned());
      Ok(())
    }
  }
}
