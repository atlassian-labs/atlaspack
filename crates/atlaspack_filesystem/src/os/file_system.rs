use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::OsMetadata;
use crate::file_system::FileSystem;
use crate::file_system::Metadata;
use crate::file_system::Permissions;

pub struct OsFileSystem {}

impl FileSystem for OsFileSystem {
  fn canonicalize<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<std::path::PathBuf> {
    fs::canonicalize(path)
  }

  fn read_link<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<std::path::PathBuf> {
    fs::read_link(path)
  }

  fn copy<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    &self,
    from: P,
    to: Q,
  ) -> std::io::Result<u64> {
    fs::copy(from, to)
  }

  fn create_dir<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<()> {
    fs::create_dir(path)
  }

  fn create_dir_all<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<()> {
    fs::create_dir_all(path)
  }

  fn hard_link<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    &self,
    original: P,
    link: Q,
  ) -> std::io::Result<()> {
    fs::hard_link(original, link)
  }

  fn metadata<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<Box<dyn Metadata>> {
    let metadata = fs::metadata(path)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn read<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<Vec<u8>> {
    fs::read(path)
  }

  fn read_dir<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<std::fs::ReadDir> {
    fs::read_dir(path)
  }

  fn read_to_string<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<String> {
    fs::read_to_string(path)
  }

  fn remove_dir<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<()> {
    fs::remove_dir(path)
  }

  fn remove_dir_all<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<()> {
    fs::remove_dir_all(path)
  }

  fn remove_file<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<()> {
    fs::remove_file(path)
  }

  fn rename<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    &self,
    from: P,
    to: Q,
  ) -> std::io::Result<()> {
    fs::rename(from, to)
  }

  fn set_permissions<P: AsRef<std::path::Path>, T: Permissions>(
    &self,
    path: P,
    perm: T,
  ) -> std::io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(perm.mode()))
  }

  fn symlink_metadata<P: AsRef<std::path::Path>>(
    &self,
    path: P,
  ) -> std::io::Result<Box<dyn Metadata>> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn write<P: AsRef<std::path::Path>, C: AsRef<[u8]>>(
    &self,
    path: P,
    contents: C,
  ) -> std::io::Result<()> {
    fs::write(path, contents)
  }
}
