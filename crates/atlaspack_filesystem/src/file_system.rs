//! Abstraction of the file system
//!
//! This module contains traits and implementations that replicate
//! the functionality of the Rust standard library file system

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

pub type FileSystemRef = Arc<dyn FileSystem + Send + Sync>;

pub trait FileSystem {
  fn cwd(&self) -> io::Result<PathBuf> {
    unimplemented!()
  }

  fn exists(&self, _path: &Path) -> io::Result<bool> {
    unimplemented!()
  }

  // Rust std::fs below
  fn canonicalize(&self, _path: &Path) -> io::Result<PathBuf> {
    unimplemented!()
  }

  fn read_link(&self, _path: &Path) -> io::Result<PathBuf> {
    unimplemented!()
  }

  fn copy(&self, _from: &Path, _to: &Path) -> io::Result<u64> {
    unimplemented!()
  }

  fn create_dir(&self, _path: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn hard_link(&self, _original: &Path, _link: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn metadata(&self, _path: &Path) -> io::Result<Box<dyn Metadata>> {
    unimplemented!()
  }

  fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
    unimplemented!()
  }

  fn read_dir(&self, _path: &Path) -> io::Result<fs::ReadDir> {
    unimplemented!()
  }

  fn read_to_string(&self, _path: &Path) -> io::Result<String> {
    unimplemented!()
  }

  fn remove_dir(&self, _path: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn remove_dir_all(&self, _path: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn remove_file(&self, _path: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
    unimplemented!()
  }

  fn set_permissions(&self, _path: &Path, _perm: &dyn Permissions) -> io::Result<()> {
    unimplemented!()
  }

  fn symlink_metadata(&self, _path: &Path) -> io::Result<Box<dyn Metadata>> {
    unimplemented!()
  }

  fn write(&self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
    unimplemented!()
  }
}

pub trait Metadata {
  fn accessed(&self) -> io::Result<SystemTime> {
    unimplemented!()
  }

  fn created(&self) -> io::Result<SystemTime> {
    unimplemented!()
  }

  fn file_type(&self) -> Box<dyn FileType> {
    unimplemented!()
  }

  fn is_dir(&self) -> bool {
    unimplemented!()
  }

  fn is_file(&self) -> bool {
    unimplemented!()
  }

  fn is_symlink(&self) -> bool {
    unimplemented!()
  }

  fn modified(&self) -> io::Result<SystemTime> {
    unimplemented!()
  }

  fn permissions(&self) -> Box<dyn Permissions> {
    unimplemented!()
  }

  fn len(&self) -> u64 {
    unimplemented!()
  }
}

pub trait Permissions {
  fn readonly(&self) -> bool {
    unimplemented!()
  }

  fn set_readonly(&mut self, _readonly: bool) {
    unimplemented!()
  }

  #[cfg(target_family = "unix")]
  fn mode(&self) -> u32 {
    unimplemented!()
  }

  #[cfg(target_family = "unix")]
  fn set_mode(&mut self, _mode: u32) {
    unimplemented!()
  }
}

pub trait FileType {
  fn is_dir(&self) -> bool {
    unimplemented!()
  }

  fn is_file(&self) -> bool {
    unimplemented!()
  }

  fn is_symlink(&self) -> bool {
    unimplemented!()
  }
}
