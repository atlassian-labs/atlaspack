use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

pub struct TempDir(PathBuf);

impl TempDir {
  pub fn new(base: &Path) -> anyhow::Result<Self> {
    if !fs::exists(base)? {
      fs::create_dir(base)?;
    }

    let random = format!("{}", rand::random::<i32>());

    let target = base.join(&random);
    if !fs::exists(&target)? {
      fs::create_dir(&target)?;
    }

    Ok(Self(target))
  }
}

impl AsRef<Path> for TempDir {
  fn as_ref(&self) -> &Path {
    &self.0
  }
}

impl Deref for TempDir {
  type Target = PathBuf;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl Drop for TempDir {
  fn drop(&mut self) {
    fs::remove_dir_all(&self.0).ok();
  }
}
