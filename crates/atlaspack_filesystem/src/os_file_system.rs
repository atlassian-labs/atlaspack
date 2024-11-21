use std::path::Path;
use std::path::PathBuf;

use canonicalize::canonicalize;

use crate::{FileSystem, FileSystemRealPathCache};

mod canonicalize;

#[derive(Default, Debug)]
pub struct OsFileSystem;

impl FileSystem for OsFileSystem {
  fn cwd(&self) -> std::io::Result<PathBuf> {
    std::env::current_dir()
  }

  fn canonicalize(&self, path: &Path, cache: &FileSystemRealPathCache) -> std::io::Result<PathBuf> {
    canonicalize(path, cache)
  }

  fn canonicalize_base(&self, path: &Path) -> std::io::Result<PathBuf> {
    canonicalize(path, &Default::default())
  }

  fn create_directory(&self, path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
  }

  fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
  }

  fn read_dir(&self, path: &Path) -> std::io::Result<std::fs::ReadDir> {
    std::fs::read_dir(path)
  }

  fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
  }

  fn is_file(&self, path: &Path) -> bool {
    let path: &Path = path;
    path.is_file()
  }

  fn is_dir(&self, path: &Path) -> bool {
    let path: &Path = path;
    path.is_dir()
  }
}
