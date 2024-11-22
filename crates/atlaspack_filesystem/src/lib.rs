use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_shared_map::SharedHashMap;

/// In-memory file-system for testing
pub mod in_memory_file_system;

pub mod search;

/// File-system implementation using std::fs and a canonicalize cache
pub mod os_file_system;

/// FileSystem abstraction instance
///
/// This should be `OsFileSystem` for non-testing environments and `InMemoryFileSystem` for testing.
pub type FileSystemRef = Arc<dyn FileSystem + Send + Sync>;

pub type FileSystemRealPathCache = SharedHashMap<PathBuf, Option<PathBuf>>;

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
  fn read_dir(&self, path: &Path) -> std::io::Result<std::fs::ReadDir>;
  fn read_to_string(&self, path: &Path) -> std::io::Result<String>;
  fn is_file(&self, path: &Path) -> bool;
  fn is_dir(&self, path: &Path) -> bool;
}
