use parking_lot::RwLock;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::ResolverError;
use crate::path::normalize_path;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum FileCreateInvalidation {
  Path(PathBuf),
  FileName { file_name: String, above: PathBuf },
  Glob(String),
}

#[derive(Default, Debug)]
pub struct Invalidations {
  pub invalidate_on_file_create:
    RwLock<HashSet<FileCreateInvalidation, xxhash_rust::xxh3::Xxh3Builder>>,
  pub invalidate_on_file_change: RwLock<HashSet<PathBuf, xxhash_rust::xxh3::Xxh3Builder>>,
  pub invalidate_on_startup: AtomicBool,
}

impl Invalidations {
  pub fn invalidate_on_file_create(&self, path: &Path) {
    self
      .invalidate_on_file_create
      .write()
      .insert(FileCreateInvalidation::Path(normalize_path(path)));
  }

  pub fn invalidate_on_file_create_above<S: Into<String>>(&self, file_name: S, above: &Path) {
    self
      .invalidate_on_file_create
      .write()
      .insert(FileCreateInvalidation::FileName {
        file_name: file_name.into(),
        above: normalize_path(above),
      });
  }

  pub fn invalidate_on_glob_create<S: Into<String>>(&self, glob: S) {
    self
      .invalidate_on_file_create
      .write()
      .insert(FileCreateInvalidation::Glob(glob.into()));
  }

  pub fn invalidate_on_file_change(&self, invalidation: &Path) {
    self
      .invalidate_on_file_change
      .write()
      .insert(normalize_path(invalidation));
  }

  pub fn invalidate_on_startup(&self) {
    self.invalidate_on_startup.store(true, Ordering::Relaxed)
  }

  pub fn extend(&self, other: &Invalidations) {
    let mut invalidate_on_file_create = self.invalidate_on_file_create.write();
    for f in other.invalidate_on_file_create.read().iter() {
      invalidate_on_file_create.insert(f.clone());
    }

    let mut invalidate_on_file_change = self.invalidate_on_file_change.write();
    for f in other.invalidate_on_file_change.read().iter() {
      invalidate_on_file_change.insert(f.clone());
    }

    if other.invalidate_on_startup.load(Ordering::Relaxed) {
      self.invalidate_on_startup();
    }
  }

  pub fn read<V, F: FnOnce() -> Result<V, ResolverError>>(
    &self,
    path: &Path,
    f: F,
  ) -> Result<V, ResolverError> {
    match f() {
      Ok(v) => {
        self.invalidate_on_file_change(path);
        Ok(v)
      }
      Err(e) => {
        if matches!(e, ResolverError::IOError(..)) {
          self.invalidate_on_file_create(path);
        }
        Err(e)
      }
    }
  }
}
