use std::fs::{create_dir_all, remove_dir_all};
use std::path::Path;
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam::channel::{Receiver, Sender};
use heed::types::{Bytes, Str};
use heed::EnvOpenOptions;
use heed::{Env, RoTxn, RwTxn};
use heed::{EnvFlags, MdbError};
use napi_derive::napi;
use rayon::prelude::*;

use crate::NativeEntry;

type Result<R> = std::result::Result<R, DatabaseWriterError>;

#[derive(thiserror::Error, Debug)]
pub enum DatabaseWriterError {
  #[error("heed error: {0}")]
  Heed(#[from] heed::Error),
  #[error("IO error: {0}")]
  IO(#[from] std::io::Error),
  #[error("Failed to decompress entry {0}")]
  Decompress(#[from] lz4_flex::block::DecompressError),
  #[error("Failed to compress entry {0}")]
  Compress(#[from] lz4_flex::block::CompressError),
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
#[napi(object)]
pub struct LMDBOptions {
  /// The database directory path
  pub path: String,
  /// If enabled, the database writer will set the following flags:
  ///
  /// * MAP_ASYNC - "use asynchronous msync when MDB_WRITEMAP is used"
  /// * NO_SYNC - "don't fsync after commit"
  /// * NO_META_SYNC - "don't fsync metapage after commit"
  ///
  /// `MDB_WRITEMAP` is on by default.
  pub async_writes: bool,
  /// The mmap size, this corresponds to [`mdb_env_set_mapsize`](http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5)
  /// if this isn't set it'll default to around 10MB.
  pub map_size: Option<f64>,
}

/// This is a message passing handle into the writer thread.
///
/// There is always a single writer thread per database.
pub struct DatabaseWriterHandle {
  tx: Sender<DatabaseWriterMessage>,
  thread_handle: Option<JoinHandle<()>>,
}

impl DatabaseWriterHandle {
  /// Send a message into the writer thread.
  pub fn send(
    &self,
    message: DatabaseWriterMessage,
  ) -> std::result::Result<(), crossbeam::channel::SendError<DatabaseWriterMessage>> {
    self.tx.send(message)
  }

  pub fn thread_id(&self) -> std::thread::ThreadId {
    self.thread_handle.as_ref().unwrap().thread().id()
  }

  pub fn join(&mut self) -> std::thread::Result<()> {
    if let Some(thread_handle) = self.thread_handle.take() {
      thread_handle.join()
    } else {
      Ok(())
    }
  }
}

impl Drop for DatabaseWriterHandle {
  fn drop(&mut self) {
    let _ = self.tx.send(DatabaseWriterMessage::Stop);
    let _ = self.join();
  }
}

/// Open the database and start the writer thread. Two handles are returned:
///
/// * A raw DB handle that can be used for synchronous reads
/// * A writer handle that can be used to send messages to the writer thread
///
/// The writer handle should be used to create write transactions shared across
/// Node.js threads.
pub fn start_make_database_writer(
  options: &LMDBOptions,
) -> Result<(DatabaseWriterHandle, Arc<DatabaseWriter>)> {
  let (tx, rx) = crossbeam::channel::unbounded();
  let writer = Arc::new(DatabaseWriter::new(options)?);

  let thread_handle = std::thread::spawn({
    let writer = writer.clone();
    move || {
      run_database_writer(rx, writer);
    }
  });

  Ok((
    DatabaseWriterHandle {
      tx,
      thread_handle: Some(thread_handle),
    },
    writer,
  ))
}

/// Main-loop for the database writer thread
fn run_database_writer(rx: Receiver<DatabaseWriterMessage>, writer: Arc<DatabaseWriter>) {
  tracing::debug!(
    "Starting database writer thread {:?}",
    std::thread::current().id()
  );
  let mut current_transaction: Option<RwTxn> = None;

  while let Ok(msg) = rx.recv() {
    if handle_message(&writer, &mut current_transaction, msg) {
      break;
    }
  }

  if let Some(txn) = current_transaction {
    let _ = txn.commit();
  }

  tracing::debug!(
    "Database writer thread {:?} exited",
    std::thread::current().id()
  );
}

#[allow(clippy::needless_lifetimes)]
fn handle_message<'a, 'b>(
  writer: &'a DatabaseWriter,
  current_transaction: &'b mut Option<RwTxn<'a>>,
  msg: DatabaseWriterMessage,
) -> bool {
  match msg {
    DatabaseWriterMessage::Get { key, resolve } => {
      let run = || {
        if let Some(txn) = &current_transaction {
          let result = writer.get(txn, &key)?.map(|d| d.to_owned());
          Ok(result)
        } else {
          let txn = writer.environment.read_txn()?;
          let result = writer.get(&txn, &key)?.map(|d| d.to_owned());
          txn.commit()?;
          Ok(result)
        }
      };
      let result = run();
      resolve(result.map(|o| o.map(|d| d.to_owned())));
    }
    DatabaseWriterMessage::Put {
      value,
      resolve,
      key,
    } => {
      let mut run = || {
        if let Some(txn) = current_transaction {
          writer.put(txn, &key, &value)?;
          Ok(())
        } else {
          let mut txn = writer.environment.write_txn()?;
          writer.put(&mut txn, &key, &value)?;
          txn.commit()?;
          Ok(())
        }
      };
      let result = run();
      resolve(result);
    }
    DatabaseWriterMessage::Stop => {
      tracing::debug!(
        "Stopping writer thread {:?}...",
        std::thread::current().id()
      );

      if let Some(txn) = current_transaction.take() {
        if let Err(e) = txn.commit() {
          tracing::error!("Failed to commit trailing transaction on shutdown: {e}");
        }
      }
      return true;
    }
    DatabaseWriterMessage::StartTransaction { resolve } => {
      if current_transaction.is_none() {
        let mut run = || {
          *current_transaction = Some(writer.environment.write_txn()?);
          Ok(())
        };
        resolve(run())
      } else {
        resolve(Ok(()))
      }
    }
    DatabaseWriterMessage::CommitTransaction { resolve } => {
      if let Some(txn) = current_transaction.take() {
        resolve(txn.commit().map_err(DatabaseWriterError::from))
      } else {
        resolve(Ok(()))
      }
    }
    DatabaseWriterMessage::PutMany { entries, resolve } => {
      let mut run = || {
        let compressed_entries: Vec<Vec<u8>> = entries
          .par_iter()
          .map(|entry| lz4_flex::block::compress_prepend_size(&entry.value))
          .collect();

        let mut txn = if let Some(txn) = current_transaction {
          RwTransaction::Borrowed(txn)
        } else {
          let txn = writer.environment.write_txn()?;
          RwTransaction::Owned(txn)
        };

        for (NativeEntry { key, .. }, compressed_value) in entries.iter().zip(compressed_entries) {
          writer
            .database
            .put(txn.deref_mut(), key, &compressed_value)?;
        }

        if let RwTransaction::Owned(txn) = txn {
          txn.commit()?;
        }

        Ok(())
      };
      let result = run();
      resolve(result);
    }
    DatabaseWriterMessage::Delete { key, resolve } => {
      let mut run = || {
        let mut txn = if let Some(txn) = current_transaction {
          RwTransaction::Borrowed(txn)
        } else {
          let txn = writer.environment.write_txn()?;
          RwTransaction::Owned(txn)
        };

        writer.delete(txn.deref_mut(), &key)?;

        if let RwTransaction::Owned(txn) = txn {
          txn.commit()?;
        }

        Ok(())
      };

      resolve(run());
    }
  }
  false
}

type ResolveCallback<T> = Box<dyn FnOnce(Result<T>) + Send>;

pub enum DatabaseWriterMessage {
  Get {
    key: String,
    resolve: ResolveCallback<Option<Vec<u8>>>,
  },
  Put {
    key: String,
    value: Vec<u8>,
    resolve: ResolveCallback<()>,
  },
  PutMany {
    entries: Vec<NativeEntry>,
    resolve: ResolveCallback<()>,
  },
  Delete {
    key: String,
    resolve: ResolveCallback<()>,
  },
  StartTransaction {
    resolve: ResolveCallback<()>,
  },
  CommitTransaction {
    resolve: ResolveCallback<()>,
  },
  Stop,
}

pub enum RwTransaction<'a, 'b> {
  Owned(RwTxn<'b>),
  Borrowed(&'a mut RwTxn<'b>),
}

impl<'b> RwTransaction<'_, 'b> {
  #[allow(clippy::should_implement_trait)]
  pub fn deref_mut(&mut self) -> &mut RwTxn<'b> {
    match self {
      RwTransaction::Borrowed(txn) => txn,
      RwTransaction::Owned(txn) => txn,
    }
  }
}

pub enum Transaction<'a, 'b> {
  Owned(RoTxn<'b>),
  Borrowed(&'a RoTxn<'b>),
}

impl<'b> Transaction<'_, 'b> {
  #[allow(clippy::should_implement_trait)]
  pub fn deref(&self) -> &RoTxn<'b> {
    match self {
      Transaction::Borrowed(txn) => txn,
      #[allow(clippy::needless_borrow)]
      Transaction::Owned(txn) => &txn,
    }
  }
}

/// Wraps a LMDB database environment.
///
/// This is thread-safe and can be shared across threads. LMDB itself will
/// manage locks.
///
/// It's important that batch writes are done within a single write transaction.
///
/// Entries are individually compressed on read/write. In the future we may
/// want to expose batch write methods that run compression in multiple threads.
///
/// The JavaScript writer thread [`DatabaseWriterHandle`] is doing this
/// internally. The most basic usecases should be covered by this simplistic
/// API.
pub struct DatabaseWriter {
  environment: Env,
  database: heed::Database<Str, Bytes>,
}

impl DatabaseWriter {
  /// Create a new [`DatabaseWriter`] handle see [`LMDBOptions`] for
  /// documentation on the settings.
  pub fn new(options: &LMDBOptions) -> Result<Self> {
    let mut env_open_options = EnvOpenOptions::new();
    let mut flags = EnvFlags::empty();
    let path = Path::new(&options.path);

    create_dir_all(path)?;

    flags.set(EnvFlags::MAP_ASYNC, options.async_writes);
    flags.set(EnvFlags::NO_SYNC, options.async_writes);
    flags.set(EnvFlags::WRITE_MAP, true);
    flags.set(EnvFlags::NO_READ_AHEAD, false);
    flags.set(EnvFlags::NO_META_SYNC, options.async_writes);

    // http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5
    // max DB size that will be memory mapped
    if let Some(map_size) = options.map_size {
      env_open_options.map_size(map_size as usize);
    }

    let environment = unsafe {
      env_open_options.flags(flags);

      let mut env = env_open_options.open(path);
      if let Err(heed::Error::Mdb(MdbError::Invalid)) = env {
        // Remove invalid v2 caches and retry opening the database
        tracing::warn!("Clearing incompatible cache {}", path.display());
        remove_dir_all(path)?;
        create_dir_all(path)?;
        env = env_open_options.open(path);
      }

      env
    }?;

    // If processes are terminated, they might leave stale readers on the DB
    // the database will be unusable after it reaches a certain number of
    // readers, so clean-up is useful on start-up.
    environment.clear_stale_readers()?;

    let mut write_txn = environment.write_txn()?;
    let database = environment.create_database(&mut write_txn, None)?;

    write_txn.commit()?;

    Ok(Self {
      database,
      environment,
    })
  }

  /// Check if an entry exists in the database
  pub fn has(&self, txn: &RoTxn, key: &str) -> Result<bool> {
    Ok(self.database.get(txn, key)?.is_some())
  }

  pub fn keys(&self, txn: &RoTxn, skip: usize, limit: usize) -> Result<Vec<String>> {
    self
      .database
      .iter(txn)?
      .skip(skip)
      .take(limit)
      .map(|entry| -> Result<_> { Ok(entry?.0.to_string()) })
      .collect()
  }

  /// Read an entry and decompress it
  pub fn get(&self, txn: &RoTxn, key: &str) -> Result<Option<Vec<u8>>> {
    if let Some(result) = self.database.get(txn, key)? {
      let output_buffer = lz4_flex::block::decompress_size_prepended(result)?;
      Ok(Some(output_buffer))
    } else {
      Ok(None)
    }
  }

  /// Compress an entry and store it
  pub fn put(&self, txn: &mut RwTxn, key: &str, data: &[u8]) -> Result<()> {
    let compressed_data = lz4_flex::block::compress_prepend_size(data);
    self.database.put(txn, key, &compressed_data)?;
    Ok(())
  }

  pub fn delete(&self, txn: &mut RwTxn, key: &str) -> Result<()> {
    self.database.delete(txn, key)?;
    Ok(())
  }

  /// Create a write transaction
  pub fn write_txn(&self) -> heed::Result<RwTxn> {
    self.environment.write_txn()
  }

  /// Create a read transaction
  pub fn read_txn(&self) -> heed::Result<RoTxn> {
    self.environment.read_txn()
  }

  /// Create a static read transaction that owns a reference counted copy of
  /// the database environment
  pub fn static_read_txn(&self) -> heed::Result<RoTxn<'static>> {
    self.environment.clone().static_read_txn()
  }

  /// Compact the database to the target path
  pub fn compact(&self, target_path: &Path) -> Result<()> {
    self
      .environment
      .copy_to_file(target_path, heed::CompactionOption::Enabled)?;

    Ok(())
  }

  pub fn path(&self) -> &Path {
    self.environment.path()
  }
}

#[cfg(test)]
mod tests {
  use std::env::temp_dir;
  use std::sync::mpsc::channel;

  use super::*;

  fn random() -> String {
    let value = rand::random::<i32>();
    format!("{value}")
  }

  #[test]
  fn database_writer_can_read_and_write() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join(random())
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);

    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };

    let writer = DatabaseWriter::new(&options).unwrap();
    let mut write_txn = writer.environment.write_txn().unwrap();
    writer
      .put(&mut write_txn, "key", &[1, 2, 3, 3, 3, 3, 3, 3, 4])
      .unwrap();
    write_txn.commit().unwrap();

    let read_txn = writer.environment.read_txn().unwrap();
    let value = writer.get(&read_txn, "key").unwrap().unwrap();
    assert_eq!(&value, &vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    drop(read_txn);
    let read_txn = writer.environment.read_txn().unwrap();
    let value = writer.get(&read_txn, "other-key").unwrap();
    assert_eq!(&value, &None);
  }

  #[test]
  fn database_writer_thread_write() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join(random())
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);

    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };

    let (writer, _) = start_make_database_writer(&options).unwrap();
    put_sync(&writer, "key1", vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    put_sync(&writer, "key2", vec![1, 2, 3]);
  }

  #[test]
  fn database_writer_thread_read_after_write() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join(random())
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);

    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };

    let (writer, _) = start_make_database_writer(&options).unwrap();
    put_sync(&writer, "key1", vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    let result = get_sync(&writer, "key1");
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));
    put_sync(&writer, "key2", vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    let result = get_sync(&writer, "key2");
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));
  }

  #[test]
  fn database_writer_thread_read_after_bulk_write() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join(random())
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);

    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };

    let (writer, _) = start_make_database_writer(&options).unwrap();
    let (tx, rx) = channel();
    writer
      .send(DatabaseWriterMessage::PutMany {
        entries: vec![
          NativeEntry {
            key: "key1".into(),
            value: vec![1, 2, 3, 3, 3, 3, 3, 3, 4],
          },
          NativeEntry {
            key: "key2".into(),
            value: vec![1, 2, 3, 3, 3, 3, 3, 3, 4],
          },
        ],
        resolve: Box::new(move |result| {
          tx.send(result).unwrap();
        }),
      })
      .unwrap();
    rx.recv().unwrap().unwrap();

    let result = get_sync(&writer, "key1");
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));
    put_sync(&writer, "key2", vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    let result = get_sync(&writer, "key2");
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));
  }

  #[test]
  fn database_writer_thread_read_within_transaction() {
    let db_path = temp_dir()
      .join("lmdb-js-lite")
      .join(random())
      .join("lmdb-cache-tests.db");
    let _ = std::fs::remove_dir_all(&db_path);

    let options = LMDBOptions {
      path: db_path.to_str().unwrap().to_string(),
      async_writes: false,
      map_size: None,
    };

    let (writer, reader) = start_make_database_writer(&options).unwrap();
    writer
      .send(DatabaseWriterMessage::StartTransaction {
        resolve: Box::new(|_| {}),
      })
      .unwrap();
    put_sync(&writer, "key1", vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    let result = get_sync(&writer, "key1");
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));
    put_sync(&writer, "key2", vec![1, 2, 3, 3, 3, 3, 3, 3, 4]);
    let result = get_sync(&writer, "key2");
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));

    let main_txn = reader.read_txn().unwrap();
    let result = reader.get(&main_txn, "key1").unwrap();
    assert_eq!(result, None);
    drop(main_txn);

    // After commit
    let (tx, rx) = channel();
    writer
      .send(DatabaseWriterMessage::CommitTransaction {
        resolve: Box::new(move |result| tx.send(result).unwrap()),
      })
      .unwrap();
    rx.recv().unwrap().unwrap();

    let main_txn = reader.read_txn().unwrap();
    let result = reader.get(&main_txn, "key1").unwrap();
    assert_eq!(result, Some(vec![1, 2, 3, 3, 3, 3, 3, 3, 4]));
  }

  fn put_sync(writer: &DatabaseWriterHandle, key: impl Into<String>, value: Vec<u8>) {
    let (tx, rx) = channel();
    writer
      .send(DatabaseWriterMessage::Put {
        key: key.into(),
        value,
        resolve: Box::new(move |result| {
          tx.send(result).unwrap();
        }),
      })
      .unwrap();
    rx.recv().unwrap().unwrap();
  }

  fn get_sync(writer: &DatabaseWriterHandle, key: impl Into<String>) -> Option<Vec<u8>> {
    let (tx, rx) = channel();
    writer
      .send(DatabaseWriterMessage::Get {
        key: key.into(),
        resolve: Box::new(move |result| {
          tx.send(result).unwrap();
        }),
      })
      .unwrap();
    rx.recv().unwrap().unwrap()
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
      buffer.push(rand::random());
    }
    // 1GB writes +/-
    for i in 0..1024 {
      let mut write_txn = read.environment.write_txn().unwrap();
      let error = (|| -> std::result::Result<(), DatabaseWriterError> {
        read.put(&mut write_txn, &format!("{i}"), &buffer)?;
        write_txn.commit()?;
        Ok(())
      })();
      if let Err(DatabaseWriterError::Heed(heed::Error::Mdb(heed::MdbError::MapFull))) = error {
        current_size *= 2;
        tracing::info!("Resizing database {current_size}");
        unsafe { read.environment.resize(current_size).unwrap() }
      }
    }
  }
}
