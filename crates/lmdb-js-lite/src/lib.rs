//! This crate implements a LMDB wrapper for Node.js using N-API.
//!
//! The wrapper is designed to be used from multi-threaded Node.js applications
//! that use transactions sparingly or not at all.
//!
//! A global mutex holds a map of reference counted open database handles.
//!
//! This is because, by contract, we can’t open the same database multiple times
//! on a single process, even though we can access it from multiple
//! threads/processes.
//!
//! When JavaScript opens a database, the handle is created and added to the
//! map.
//!
//! Each database handle consists of 2 parts, the native LMDB handle and a
//! message channel into a writer thread. These are the respectively:
//!
//! - [`DatabaseWriter`] - The native LMDB handle
//! - [`DatabaseWriterHandle`] - The message channel onto a writer thread
//!
//! Because we want to avoid blocking JavaScript threads waiting on write
//! compression and acquiring the write lock, all writes are sent to a single
//! writer thread
//!
//! This means currently compression runs single-threaded, which is not ideal.
//!
//! Reads will never lock, and it’s faster to de-compress on the main-thread
//! than the writer thread, since we avoid both message passing overhead,
//! waiting on other threads and creating JavaScript promises for the reads.
//!
//! We use a rust implementation of lz4, which is many times faster than the
//! native version - [`lz4_flex`].
//!
//! If `async_writes` is turned on, we turn off fsync / msync after commits ;
//! this means that the database will have lower durability guarantees, but
//! it should still be consistent in memory and within transactions.
#![deny(clippy::all)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};

use anyhow::anyhow;
use lazy_static::lazy_static;
use napi::bindgen_prelude::Env;
use napi::JsUnknown;
use napi_derive::napi;
use tracing::Level;

pub use crate::writer::LMDBOptions;
use crate::writer::{
  start_make_database_writer, DatabaseWriter, DatabaseWriterError, DatabaseWriterHandle,
  DatabaseWriterMessage,
};

pub mod writer;

#[cfg(not(test))]
type Buffer = napi::bindgen_prelude::Buffer;
#[cfg(test)]
type Buffer = Vec<u8>;

fn napi_error(err: impl Debug) -> napi::Error {
  napi::Error::from_reason(format!("[napi] {err:?}"))
}

#[derive(Clone)]
pub struct DatabaseHandle {
  /// This is a handle into the writer thread
  writer_thread_handle: Arc<DatabaseWriterHandle>,
  /// This is a raw handle to the LMDB database
  database: Arc<DatabaseWriter>,
}

impl DatabaseHandle {
  /// This should only be used if you need to share transaction state with
  /// JavaScript writers, as in, use the same transaction for multiple threads.
  pub fn writer_thread_handle(&self) -> &Arc<DatabaseWriterHandle> {
    &self.writer_thread_handle
  }

  /// Get the raw database handle. Prefer using this for reads and writes on
  /// native.
  pub fn database(&self) -> &Arc<DatabaseWriter> {
    &self.database
  }
}

struct LMDBGlobalState {
  /// Grows unbounded. It will not be cleaned-up as that complicates things. Opening and closing
  /// many databases on the same process will cause this to grow.
  databases: HashMap<String, Weak<DatabaseHandle>>,
}

impl LMDBGlobalState {
  fn new() -> Self {
    Self {
      databases: HashMap::new(),
    }
  }

  fn get_database(
    &mut self,
    options: LMDBOptions,
  ) -> Result<Arc<DatabaseHandle>, DatabaseWriterError> {
    if let Some(database) = self
      .databases
      .get(&options.path)
      .and_then(|database| database.upgrade())
    {
      return Ok(database);
    }
    let (writer, database) = start_make_database_writer(&options)?;
    let handle = Arc::new(DatabaseHandle {
      writer_thread_handle: Arc::new(writer),
      database,
    });
    self.databases.insert(options.path, Arc::downgrade(&handle));
    Ok(handle)
  }
}

lazy_static! {
  static ref STATE: Mutex<LMDBGlobalState> = Mutex::new(LMDBGlobalState::new());
}

#[napi]
pub fn init_tracing_subscriber() {
  let _ = tracing_subscriber::FmtSubscriber::builder()
    .with_max_level(Level::DEBUG)
    .try_init();
}

#[napi(object)]
pub struct Entry {
  pub key: String,
  pub value: Buffer,
}

pub struct NativeEntry {
  pub key: String,
  // We copy out of the buffer because it's undefined behaviour to send it across
  pub value: Vec<u8>,
}

#[napi]
pub struct LMDB {
  inner: Option<Arc<DatabaseHandle>>,
  read_transaction: Option<heed::RoTxn<'static>>,
}

#[napi]
impl LMDB {
  #[napi(constructor)]
  pub fn new(options: LMDBOptions) -> napi::Result<Self> {
    let mut state = STATE
      .lock()
      .map_err(|_| napi::Error::from_reason("LMDB State mutex is poisoned"))?;
    let database = state.get_database(options).map_err(napi_error)?;
    Ok(Self {
      inner: Some(database),
      read_transaction: None,
    })
  }

  #[napi(ts_return_type = "Promise<Buffer | null | undefined>")]
  pub fn get(&self, env: Env, key: String) -> napi::Result<napi::JsObject> {
    let database_handle = self.get_database_napi()?;
    let (deferred, promise) = env.create_deferred()?;

    database_handle
      .writer_thread_handle()
      .send(DatabaseWriterMessage::Get {
        key,
        resolve: Box::new(|value| match value {
          Ok(value) => deferred.resolve(move |_| Ok(value.map(Buffer::from))),
          Err(err) => deferred.reject(napi_error(err)),
        }),
      })
      .map_err(|err| napi_error(anyhow!("Failed to send {err}")))?;

    Ok(promise)
  }

  #[napi(ts_return_type = "Buffer | null")]
  pub fn get_sync(&self, env: Env, key: String) -> napi::Result<JsUnknown> {
    let database_handle = self.get_database_napi()?;
    let database = &database_handle.database;

    let txn = if let Some(txn) = &self.read_transaction {
      writer::Transaction::Borrowed(txn)
    } else {
      writer::Transaction::Owned(
        database
          .read_txn()
          .map_err(|err| napi_error(anyhow!(err)))?,
      )
    };
    let buffer = database.get(txn.deref(), &key);
    let Some(buffer) = buffer.map_err(|err| napi_error(anyhow!(err)))? else {
      return Ok(env.get_null()?.into_unknown());
    };

    let size = buffer.len();
    // Empty files that exist in the db have no size
    if size == 0 {
      // We must avoid calling create_buffer with a size of 0 as it will panic
      return Ok(env.create_buffer_with_data(Vec::new())?.into_unknown());
    }

    let mut result = env.create_buffer(size)?;
    // This is faster than moving the vector in
    result.copy_from_slice(&buffer);
    Ok(result.into_unknown())
  }

  #[napi]
  pub fn get_many_sync(&self, keys: Vec<String>) -> napi::Result<Vec<Option<Buffer>>> {
    let database_handle = self.get_database_napi()?;
    let database = &database_handle.database;

    let mut results = vec![];
    let txn = database
      .read_txn()
      .map_err(|err| napi_error(anyhow!(err)))?;

    for key in keys {
      let buffer = database
        .get(&txn, &key)
        .map_err(|err| napi_error(anyhow!(err)))?
        .map(Buffer::from);
      results.push(buffer);
    }

    Ok(results)
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn put_many(&self, env: Env, entries: Vec<Entry>) -> napi::Result<napi::JsObject> {
    let database_handle = self.get_database_napi()?;
    let (deferred, promise) = env.create_deferred()?;

    let message = DatabaseWriterMessage::PutMany {
      entries: entries
        .into_iter()
        .map(|entry| NativeEntry {
          key: entry.key,
          value: entry.value.to_vec(),
        })
        .collect(),
      resolve: Box::new(|value| {
        deferred.resolve(|_| value.map_err(|err| napi_error(anyhow!("Failed to write {err}"))))
      }),
    };
    database_handle
      .writer_thread_handle()
      .send(message)
      .map_err(|err| napi_error(anyhow!("Failed to send {err}")))?;

    Ok(promise)
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn put(&self, env: Env, key: String, data: Buffer) -> napi::Result<napi::JsObject> {
    let database_handle = self.get_database_napi()?;
    // This costs us 70% over the round-trip time after arg. conversion
    let (deferred, promise) = env.create_deferred()?;

    let message = DatabaseWriterMessage::Put {
      key,
      value: data.to_vec(),
      resolve: Box::new(|value| match value {
        Ok(value) => deferred.resolve(move |_| Ok(value)),
        Err(err) => deferred.reject(napi_error(anyhow!("Failed to write {err}"))),
      }),
    };
    database_handle
      .writer_thread_handle()
      .send(message)
      .map_err(|err| napi_error(anyhow!("Failed to send {err}")))?;

    Ok(promise)
  }

  #[napi]
  pub fn put_no_confirm(&self, key: String, data: Buffer) -> napi::Result<()> {
    let database_handle = self.get_database_napi()?;

    let message = DatabaseWriterMessage::Put {
      key,
      value: data.to_vec(),
      resolve: Box::new(|_| {}),
    };
    database_handle
      .writer_thread_handle()
      .send(message)
      .map_err(|err| napi_error(anyhow!("Failed to send {err}")))?;

    Ok(())
  }

  #[napi]
  pub fn start_read_transaction(&mut self) -> napi::Result<()> {
    if self.read_transaction.is_some() {
      return Ok(());
    }
    let database_handle = self.get_database_napi()?;
    let txn = database_handle
      .database
      .static_read_txn()
      .map_err(|err| napi_error(anyhow!(err)))?;
    self.read_transaction = Some(txn);
    Ok(())
  }

  #[napi]
  pub fn commit_read_transaction(&mut self) -> napi::Result<()> {
    if let Some(txn) = self.read_transaction.take() {
      txn.commit().map_err(|err| napi_error(anyhow!(err)))?;
      Ok(())
    } else {
      Ok(())
    }
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn start_write_transaction(&self, env: Env) -> napi::Result<napi::JsObject> {
    let database_handle = self.get_database_napi()?;
    let (deferred, promise) = env.create_deferred()?;

    let message = DatabaseWriterMessage::StartTransaction {
      resolve: Box::new(|_| deferred.resolve(|_| Ok(()))),
    };
    database_handle
      .writer_thread_handle()
      .send(message)
      .map_err(|err| napi_error(anyhow!("Failed to send {err}")))?;

    Ok(promise)
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn commit_write_transaction(&self, env: Env) -> napi::Result<napi::JsObject> {
    let database_handle = self.get_database_napi()?;
    let (deferred, promise) = env.create_deferred()?;

    let message = DatabaseWriterMessage::CommitTransaction {
      resolve: Box::new(|_| deferred.resolve(|_| Ok(()))),
    };
    database_handle
      .writer_thread_handle()
      .send(message)
      .map_err(|err| napi_error(anyhow!("Failed to send {err}")))?;

    Ok(promise)
  }

  #[napi]
  pub fn close(&mut self) {
    self.inner = None;
  }
}

impl LMDB {
  pub fn get_database_napi(&self) -> napi::Result<&Arc<DatabaseHandle>> {
    let inner = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("[napi] Trying to get a closed database"))?;
    Ok(inner)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand::random;
  use std::env::temp_dir;
  use std::sync::mpsc::channel;

  #[test]
  fn create_database() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join("create_database")
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);
    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };
    let mut lmdb = LMDB::new(options).unwrap();
    lmdb.close();
  }

  #[test]
  fn consistency_test() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join("consistency_test")
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);
    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };
    let (write, read) = start_make_database_writer(&options).unwrap();

    let (tx, rx) = channel();
    write
      .send(DatabaseWriterMessage::StartTransaction {
        resolve: Box::new(|_| {}),
      })
      .unwrap();
    write
      .send(DatabaseWriterMessage::Put {
        key: String::from("key"),
        value: vec![1, 2, 3, 4],
        resolve: Box::new(|_| {}),
      })
      .unwrap();
    // If we don't commit the reader will not see the writes.
    write
      .send(DatabaseWriterMessage::CommitTransaction {
        resolve: Box::new(move |_| {
          tx.send(()).unwrap();
        }),
      })
      .unwrap();
    rx.recv().unwrap();

    let read_txn = read.read_txn().unwrap();
    let value = read.get(&read_txn, "key").unwrap().unwrap();
    read_txn.commit().unwrap();
    assert_eq!(value, [1, 2, 3, 4]);
  }

  #[test]
  fn test_filling_up_the_database() {
    let _ = tracing_subscriber::fmt::try_init();
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join("test_filling_up_the_database")
      .join("lmdb-cache-tests.db");
    tracing::info!("db_path={db_path:?}");
    let _ = std::fs::remove_dir_all(&db_path);
    let mut current_size = 10485760;
    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };
    let (_, read) = start_make_database_writer(&options).unwrap();

    // 1MB entry
    let mut buffer: Vec<u8> = vec![];
    for _j in 0..(1024 * 1024) {
      buffer.push(random());
    }
    // 1GB writes +/-
    for i in 0..1024 {
      let mut write_txn = read.environment().write_txn().unwrap();
      let error = (|| -> Result<(), DatabaseWriterError> {
        read.put(&mut write_txn, &format!("{i}"), &buffer)?;
        write_txn.commit()?;
        Ok(())
      })();
      if let Err(DatabaseWriterError::HeedError(heed::Error::Mdb(heed::MdbError::MapFull))) = error
      {
        current_size *= 2;
        tracing::info!("Resizing database {current_size}");
        unsafe { read.environment().resize(current_size).unwrap() }
      }
    }
  }
}
