#![allow(unused)]

use std::path::Path;

pub fn soft_link(original: &Path, link: &Path) -> anyhow::Result<()> {
  #[cfg(unix)]
  std::os::unix::fs::symlink(original, link)?;

  #[cfg(windows)]
  std::os::windows::fs::symlink_dir(original, link)?;

  Ok(())
}

pub fn hard_link_or_copy(original: &Path, link: &Path) -> anyhow::Result<()> {
  match std::fs::hard_link(original, link) {
    Ok(_) => 0,
    Err(_) => std::fs::copy(original, link)?,
  };
  Ok(())
}
