use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use super::canonicalize::canonicalize;
use super::canonicalize::FileSystemRealPathCache;
use super::OsMetadata;
use crate::file_system::FileSystem;
use crate::file_system::Metadata;
use crate::file_system::Permissions;

pub struct OsFileSystem {
  cache: FileSystemRealPathCache,
}

impl Default for OsFileSystem {
  fn default() -> Self {
    Self {
      cache: Default::default(),
    }
  }
}

impl FileSystem for OsFileSystem {
  fn cwd(&self) -> io::Result<PathBuf> {
    std::env::current_dir()
  }

  fn canonicalize<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<PathBuf> {
    canonicalize(path.as_ref(), &self.cache)
  }

  fn read_link<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<PathBuf> {
    fs::read_link(path)
  }

  fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
    &self,
    from: P,
    to: Q,
  ) -> io::Result<u64> {
    fs::copy(from, to)
  }

  fn create_dir<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<()> {
    fs::create_dir(path)
  }

  fn create_dir_all<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<()> {
    fs::create_dir_all(path)
  }

  fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(
    &self,
    original: P,
    link: Q,
  ) -> io::Result<()> {
    fs::hard_link(original, link)
  }

  fn metadata<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<Box<dyn Metadata>> {
    let metadata = fs::metadata(path)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn read<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<Vec<u8>> {
    fs::read(path)
  }

  fn read_dir<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<std::fs::ReadDir> {
    fs::read_dir(path)
  }

  fn read_to_string<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<String> {
    fs::read_to_string(path)
  }

  fn remove_dir<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<()> {
    fs::remove_dir(path)
  }

  fn remove_dir_all<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<()> {
    fs::remove_dir_all(path)
  }

  fn remove_file<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<()> {
    fs::remove_file(path)
  }

  fn rename<P: AsRef<Path>, Q: AsRef<Path>>(
    &self,
    from: P,
    to: Q,
  ) -> io::Result<()> {
    fs::rename(from, to)
  }

  fn set_permissions<P: AsRef<Path>, T: Permissions>(
    &self,
    path: P,
    perm: T,
  ) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(perm.mode()))
  }

  fn symlink_metadata<P: AsRef<Path>>(
    &self,
    path: P,
  ) -> io::Result<Box<dyn Metadata>> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
    &self,
    path: P,
    contents: C,
  ) -> io::Result<()> {
    fs::write(path, contents)
  }
}
