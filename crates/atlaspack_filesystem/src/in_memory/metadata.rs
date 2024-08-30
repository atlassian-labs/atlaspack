use crate::file_system::Metadata;

pub struct MemoryMetadata {
  pub(super) inner_is_file: bool,
  pub(super) inner_is_dir: bool,
}

impl Metadata for MemoryMetadata {
  fn is_dir(&self) -> bool {
    self.inner_is_dir
  }

  fn is_file(&self) -> bool {
    self.inner_is_file
  }
}
