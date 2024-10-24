use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use thread_local::ThreadLocal;

/// In-memory file-system for testing
pub mod in_memory_file_system;

pub mod search;

/// File-system implementation using std::fs and a canonicalize cache
pub mod os_file_system;

/// FileSystem abstraction instance
///
/// This should be `OsFileSystem` for non-testing environments and `InMemoryFileSystem` for testing.
pub type FileSystemRef = Arc<dyn FileSystem + Send + Sync>;

type DefaultHasher = xxhash_rust::xxh3::Xxh3Builder;

pub struct ThreadLocalHashMap<K: Send + Eq, V: Send + Clone> {
  inner: ThreadLocal<RefCell<HashMap<K, V, DefaultHasher>>>,
}

impl<K: Send + Eq, V: Send + Clone> Default for ThreadLocalHashMap<K, V> {
  fn default() -> Self {
    Self {
      inner: ThreadLocal::new(),
    }
  }
}

impl<K: std::hash::Hash + Send + Eq, V: Send + Clone> ThreadLocalHashMap<K, V> {
  fn new() -> Self {
    Self {
      inner: ThreadLocal::new(),
    }
  }

  fn get<KR>(&self, key: &KR) -> Option<V>
  where
    KR: ?Sized,
    K: Borrow<KR>,
    KR: Eq + std::hash::Hash,
  {
    None
  }

  fn insert(&self, key: K, value: V) {}
}

pub type FileSystemRealPathCache = ThreadLocalHashMap<PathBuf, Option<PathBuf>>;

/// Trait abstracting file-system operations
/// .
///
/// ## TODO list
///
/// * [ ] Do not leak dash-map cache into calls. Instead this should be managed by implementations;
///       it should not be in the trait
/// * [ ] Do not use io results, instead use anyhow or this error
///
#[mockall::automock]
pub trait FileSystem: std::fmt::Debug {
  fn cwd(&self) -> std::io::Result<PathBuf> {
    Err(std::io::Error::new(
      std::io::ErrorKind::Other,
      "Not implemented: FileSystem::cwd",
    ))
  }

  fn canonicalize_base(&self, _path: &Path) -> std::io::Result<PathBuf> {
    Err(std::io::Error::new(
      std::io::ErrorKind::Other,
      "Not implemented: FileSystem::canonicalize_base",
    ))
  }

  fn canonicalize(
    &self,
    path: &Path,
    _cache: &FileSystemRealPathCache,
  ) -> std::io::Result<PathBuf> {
    self.canonicalize_base(path)
  }

  /// Create a directory at the specified path
  fn create_directory(&self, _path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
      std::io::ErrorKind::Other,
      "Not implemented",
    ))
  }

  fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;
  fn read_to_string(&self, path: &Path) -> std::io::Result<String>;
  fn is_file(&self, path: &Path) -> bool;
  fn is_dir(&self, path: &Path) -> bool;
}
