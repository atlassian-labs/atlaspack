use std::fs;

use crate::file_system::FileType;

pub struct FileTypeOs {
  inner: fs::FileType,
}

impl From<fs::FileType> for FileTypeOs {
  fn from(value: fs::FileType) -> Self {
    Self { inner: value }
  }
}

impl FileType for FileTypeOs {
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
