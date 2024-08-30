use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use parking_lot::RwLock;

use super::canonicalize::canonicalize_impl;
use super::MemoryMetadata;
use crate::file_system::FileSystem;
use crate::file_system::Metadata;

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
    let cwd = canonicalize_impl(&self.current_working_directory, cwd);
    let mut state = self.current_working_directory.write();
    *state = cwd;
  }
}

impl FileSystem for InMemoryFileSystem {
  fn cwd(&self) -> io::Result<PathBuf> {
    Ok(self.current_working_directory.read().clone())
  }

  fn canonicalize(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf> {
    Ok(canonicalize_impl(
      &self.current_working_directory,
      path.as_ref(),
    ))
  }

  fn create_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    let mut files = self.files.write();
    let path = canonicalize_impl(&self.current_working_directory, path.as_ref());
    files.insert(path.into(), InMemoryFileSystemEntry::Directory);
    Ok(())
  }

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>> {
    let path = canonicalize_impl(&self.current_working_directory, path.as_ref());
    let files = self.files.read();
    match files.get(&path) {
      None => Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found",
      )),
      Some(InMemoryFileSystemEntry::File { contents }) => Ok(contents.clone()),
      Some(InMemoryFileSystemEntry::Directory) => Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Path is a directory",
      )),
    }
  }

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String> {
    let path = canonicalize_impl(&self.current_working_directory, path.as_ref());
    let bytes = self.read(&path)?;
    String::from_utf8(bytes).map_err(|_| std::io::Error::other("Unable to read file as string"))
  }

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata>> {
    let path = canonicalize_impl(&self.current_working_directory, path.as_ref());
    let files = self.files.read();
    let file = files.get(&path);

    Ok(Box::new(MemoryMetadata {
      inner_is_dir: matches!(file, Some(InMemoryFileSystemEntry::Directory { .. })),
      inner_is_file: matches!(file, Some(InMemoryFileSystemEntry::File { .. })),
    }))
  }

  fn write(
    &self,
    path: &Path,
    contents: &dyn AsRef<[u8]>,
  ) -> io::Result<()> {
    let path = canonicalize_impl(&self.current_working_directory, path.as_ref());
    let mut files = self.files.write();

    files.insert(
      path.clone(),
      InMemoryFileSystemEntry::File {
        contents: contents.as_ref().to_vec(),
      },
    );

    let mut dir = path.parent();
    while let Some(path) = dir {
      files.insert(path.to_path_buf(), InMemoryFileSystemEntry::Directory);
      dir = path.parent();
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_canonicalize_noop() {
    let fs = InMemoryFileSystem::default();
    let path = root_dir().join("foo/bar");
    let result = fs.canonicalize(&path).unwrap();
    assert_eq!(result, path);
  }

  #[test]
  fn test_remove_relative_dots() {
    let fs = InMemoryFileSystem::default();
    let result = fs.canonicalize(&root_dir().join("foo/./bar")).unwrap();
    assert_eq!(result, root_dir().join("foo/bar"));
  }

  #[test]
  fn test_remove_relative_parent_dots() {
    let fs = InMemoryFileSystem::default();
    let result = fs
      .canonicalize(&root_dir().join("/foo/./bar/../baz/"))
      .unwrap();
    assert_eq!(result, root_dir().join("/foo/baz"));
  }

  #[test]
  fn test_with_cwd() {
    let fs = InMemoryFileSystem::default();
    fs.set_current_working_directory(Path::new("/other"));
    let result = fs.canonicalize(&Path::new("./foo/./bar/../baz/")).unwrap();
    assert_eq!(result, root_dir().join("/other/foo/baz"));
    assert!(result.is_absolute());
  }

  #[test]
  fn test_read_file() {
    let fs = InMemoryFileSystem::default();
    fs.write(&PathBuf::from("/foo/bar"), &"contents".as_bytes())
      .unwrap();
    let result = fs.read_to_string(&Path::new("/foo/bar")).unwrap();
    assert_eq!(result, "contents");
  }

  #[test]
  fn test_read_file_not_found() {
    let fs = InMemoryFileSystem::default();
    let result = fs.read_to_string(&Path::new("/foo/bar"));
    assert!(result.is_err());
  }

  #[test]
  fn test_is_file() {
    let cwd = PathBuf::from("atlaspack");
    let fs = InMemoryFileSystem::default();

    fs.write(&PathBuf::from("/foo/bar"), &String::default().as_bytes())
      .unwrap();

    assert!(fs.metadata(&Path::new("/foo/bar")).unwrap().is_file());
    assert!(!fs.metadata(&Path::new("/foo")).unwrap().is_file());

    fs.write(&cwd.join("src").join("a.js"), &String::default().as_bytes())
      .unwrap();

    assert!(fs
      .metadata(&cwd.join("src").join("a.js"))
      .unwrap()
      .is_file());

    assert!(fs.metadata(&cwd.join("src/a.js")).unwrap().is_file());
  }

  #[test]
  fn test_is_dir() {
    let fs = InMemoryFileSystem::default();

    fs.create_dir_all(&PathBuf::from("/foo"))
      .expect("Expected /foo directory to be created");

    assert!(fs.metadata(&Path::new("/foo")).unwrap().is_dir());
    assert!(!fs.metadata(&Path::new("/foo/bar")).unwrap().is_dir());
  }

  #[test]
  fn test_changing_the_cwd_will_correctly_resolve_files() {
    let cwd = PathBuf::from("/foo");
    let fs = InMemoryFileSystem::default();
    fs.set_current_working_directory(&cwd);

    fs.write(&PathBuf::from("bar"), &String::default().as_bytes())
      .unwrap();
    assert!(fs.metadata(&Path::new("bar")).unwrap().is_file());

    fs.set_current_working_directory(Path::new("/"));
    assert!(fs.metadata(&Path::new("/foo/bar")).unwrap().is_file());
  }

  #[cfg(target_os = "windows")]
  mod windows_tests {
    use super::*;

    #[test]
    fn test_the_prefix_will_be_carried_onto_canonicalize_paths() {
      let cwd = PathBuf::from("C:\\foo");
      let fs = InMemoryFileSystem::default();
      fs.set_current_working_directory(&cwd);
      let result = fs.canonicalize(&Path::new("\\something")).unwrap();
      assert_eq!(result, PathBuf::from("C:\\something"));
    }
  }
}
