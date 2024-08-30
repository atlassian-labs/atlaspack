use std::fs;

use crate::file_system::FileType;

pub struct OsFileType {
  inner: fs::FileType,
}

impl From<fs::FileType> for OsFileType {
  fn from(value: fs::FileType) -> Self {
    Self { inner: value }
  }
}

impl FileType for OsFileType {
  fn is_dir(&self) -> bool {
    self.inner.is_dir()
  }

  fn is_file(&self) -> bool {
    self.inner.is_file()
  }

  fn is_symlink(&self) -> bool {
    self.inner.is_symlink()
  }
}
