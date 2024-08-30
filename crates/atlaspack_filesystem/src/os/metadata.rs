use std::fs;
use std::time::SystemTime;

use super::OsFileType;
use super::OsPermissions;
use crate::file_system::FileType;
use crate::file_system::Metadata;
use crate::file_system::Permissions;

pub struct OsMetadata {
  inner: fs::Metadata,
}

impl From<fs::Metadata> for OsMetadata {
  fn from(value: fs::Metadata) -> Self {
    Self { inner: value }
  }
}

impl Metadata for OsMetadata {
  fn accessed(&self) -> std::io::Result<SystemTime> {
    self.inner.accessed()
  }

  fn created(&self) -> std::io::Result<SystemTime> {
    self.inner.created()
  }

  fn file_type(&self) -> Box<dyn FileType> {
    let file_type = self.inner.file_type();
    Box::new(OsFileType::from(file_type))
  }

  fn is_dir(&self) -> bool {
    self.inner.is_dir()
  }

  fn is_file(&self) -> bool {
    self.inner.is_file()
  }

  fn is_symlink(&self) -> bool {
    self.inner.is_symlink()
  }

  fn modified(&self) -> std::io::Result<SystemTime> {
    self.inner.modified()
  }

  fn permissions(&self) -> Box<dyn Permissions> {
    let permissions = self.inner.permissions();
    Box::new(OsPermissions::from(permissions))
  }

  fn len(&self) -> u64 {
    self.inner.len()
  }
}
