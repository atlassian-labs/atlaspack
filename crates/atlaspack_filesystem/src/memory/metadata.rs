use std::fs;
use std::time::SystemTime;

use super::FileTypeOs;
use super::PermissionsOs;
use crate::file_system::FileType;
use crate::file_system::Metadata;
use crate::file_system::Permissions;

pub struct MetadataOs {
  inner: fs::Metadata,
}

impl From<fs::Metadata> for MetadataOs {
  fn from(value: fs::Metadata) -> Self {
    Self { inner: value }
  }
}

impl Metadata for MetadataOs {
  fn accessed(&self) -> std::io::Result<SystemTime> {
    self.inner.accessed()
  }

  fn created(&self) -> std::io::Result<SystemTime> {
    self.inner.created()
  }

  fn file_type(&self) -> Box<dyn FileType> {
    let file_type = self.inner.file_type();
    Box::new(FileTypeOs::from(file_type))
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
    Box::new(PermissionsOs::from(permissions))
  }

  fn len(&self) -> u64 {
    self.inner.len()
  }
}
