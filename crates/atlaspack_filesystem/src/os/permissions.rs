use std::fs;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

use crate::file_system::Permissions;

pub struct OsPermissions {
  inner: fs::Permissions,
}

impl From<fs::Permissions> for OsPermissions {
  fn from(value: fs::Permissions) -> Self {
    Self { inner: value }
  }
}

impl Permissions for OsPermissions {
  fn readonly(&self) -> bool {
    self.inner.readonly()
  }

  fn set_readonly(
    &mut self,
    readonly: bool,
  ) {
    self.inner.set_readonly(readonly)
  }

  #[cfg(target_family = "unix")]
  fn mode(&self) -> u32 {
    self.inner.mode()
  }

  #[cfg(target_family = "unix")]
  fn set_mode(
    &mut self,
    mode: u32,
  ) {
    self.inner.set_mode(mode)
  }
}
