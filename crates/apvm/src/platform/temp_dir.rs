use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use rand::distr::Alphanumeric;
use rand::Rng;

pub struct TempDir(PathBuf);

impl TempDir {
  pub fn new(base: &Path) -> anyhow::Result<Self> {
    let random = rand::rng()
      .sample_iter(&Alphanumeric)
      .take(7)
      .map(char::from)
      .collect::<String>();

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
