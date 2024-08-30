use std::path::Path;
use std::path::PathBuf;

use crate::FileSystem;

#[derive(Debug)]
pub enum Entry<'a> {
  Directory(&'a str),
  File(&'a str),
}

pub trait FileSystemExt {
  fn find_ancestor<'a, P: AsRef<Path>>(
    &self,
    entries: &[Entry<'a>],
    from: P,
    root: P,
  ) -> Option<PathBuf>;
}

impl FileSystemExt for FileSystem {
  fn find_ancestor<'a, P: AsRef<Path>>(
    &self,
    entries: &[Entry<'a>],
    from: P,
    root: P,
  ) -> Option<PathBuf> {
    todo!()
  }
}
