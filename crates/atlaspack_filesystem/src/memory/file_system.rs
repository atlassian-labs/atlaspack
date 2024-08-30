use std::collections::HashMap;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use parking_lot::RwLock;

use crate::file_system::FileSystem;

#[cfg(not(target_os = "windows"))]
fn root_dir() -> PathBuf {
  PathBuf::from("/")
}

#[cfg(target_os = "windows")]
fn root_dir() -> PathBuf {
  PathBuf::from("C:/")
}

/// In memory implementation of a file-system entry
#[derive(Debug)]
enum InMemoryFileSystemEntry {
  File { contents: Vec<u8> },
  Directory,
}

/// In memory implementation of the `FileSystem` trait, for testing purposes.
#[derive(Debug)]
pub struct InMemoryFileSystem {
  files: RwLock<HashMap<PathBuf, InMemoryFileSystemEntry>>,
  current_working_directory: RwLock<PathBuf>,
}

impl Default for InMemoryFileSystem {
  fn default() -> Self {
    Self {
      files: Default::default(),
      current_working_directory: RwLock::new(root_dir()),
    }
  }
}

impl InMemoryFileSystem {
  /// Change the current working directory. Used for resolving relative paths.
  pub fn set_current_working_directory(
    &self,
    cwd: &Path,
  ) {
    let cwd = self.canonicalize_impl(cwd);
    let mut state = self.current_working_directory.write();
    *state = cwd;
  }

  fn canonicalize_impl(
    &self,
    path: &Path,
  ) -> PathBuf {
    let cwd = self.current_working_directory.read();
    let mut result = if path.is_absolute() {
      vec![]
    } else {
      cwd.components().collect()
    };

    let components = path.components();
    for component in components {
      match component {
        Component::Prefix(prefix) => {
          result = vec![Component::Prefix(prefix)];
        }
        Component::RootDir => {
          result.push(Component::RootDir);
        }
        Component::CurDir => {}
        Component::ParentDir => {
          result.pop();
        }
        Component::Normal(path) => {
          result.push(Component::Normal(path));
        }
      }
    }

    PathBuf::from_iter(result)
  }
}

impl FileSystem for InMemoryFileSystem {}
