use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

#[mockall::automock]
pub trait FileSystem {
  fn cwd() -> io::Result<PathBuf> {
    unimplemented!()
  }

  fn canonicalize<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<PathBuf> {
    unimplemented!()
  }

  fn read_link<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<PathBuf> {
    unimplemented!()
  }

  fn copy<P: AsRef<Path> + 'static, Q: AsRef<Path> + 'static>(
    &self,
    _from: P,
    _to: Q,
  ) -> io::Result<u64> {
    unimplemented!()
  }

  fn create_dir<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn create_dir_all<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn hard_link<P: AsRef<Path> + 'static, Q: AsRef<Path> + 'static>(
    &self,
    _original: P,
    _link: Q,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn metadata<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<Box<dyn Metadata>> {
    unimplemented!()
  }

  fn read<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<Vec<u8>> {
    unimplemented!()
  }

  fn read_dir<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<fs::ReadDir> {
    unimplemented!()
  }

  fn read_to_string<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<String> {
    unimplemented!()
  }

  fn remove_dir<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn remove_dir_all<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn remove_file<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn rename<P: AsRef<Path> + 'static, Q: AsRef<Path> + 'static>(
    &self,
    _from: P,
    _to: Q,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn set_permissions<P: AsRef<Path> + 'static, T: Permissions + 'static>(
    &self,
    _path: P,
    _perm: T,
  ) -> io::Result<()> {
    unimplemented!()
  }

  fn symlink_metadata<P: AsRef<Path> + 'static>(
    &self,
    _path: P,
  ) -> io::Result<Box<dyn Metadata>> {
    unimplemented!()
  }

  fn write<P: AsRef<Path> + 'static, C: AsRef<[u8]> + 'static>(
    &self,
    _path: P,
    _contents: C,
  ) -> io::Result<()> {
    unimplemented!()
  }
}

#[mockall::automock]
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

#[mockall::automock]
pub trait Permissions {
  fn readonly(&self) -> bool {
    unimplemented!()
  }

  fn set_readonly(
    &mut self,
    _readonly: bool,
  ) {
    unimplemented!()
  }

  #[cfg(target_family = "unix")]
  fn mode(&self) -> u32 {
    unimplemented!()
  }

  #[cfg(target_family = "unix")]
  fn set_mode(
    &mut self,
    _mode: u32,
  ) {
    unimplemented!()
  }
}

#[mockall::automock]
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
