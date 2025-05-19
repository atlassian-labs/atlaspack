#![allow(unused)]

use jwalk::WalkDir;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn soft_link(original: &Path, link: &Path) -> anyhow::Result<()> {
  #[cfg(unix)]
  std::os::unix::fs::symlink(original, link)?;

  #[cfg(windows)]
  std::os::windows::fs::symlink_dir(original, link)?;

  Ok(())
}

pub fn hard_link_or_copy<Source: AsRef<Path>, Dest: AsRef<Path>>(
  original: Source,
  link: Dest,
) -> anyhow::Result<()> {
  match std::fs::hard_link(original.as_ref(), link.as_ref()) {
    Ok(_) => 0,
    Err(_) => std::fs::copy(original, link)?,
  };
  Ok(())
}

pub fn cp_dir_recursive<S: AsRef<Path>, D: AsRef<Path>>(source: S, dest: D) -> anyhow::Result<()> {
  let source = source.as_ref();
  let dest = dest.as_ref();

  log::info!("cp_dir_recursive: {:?} -> {:?}", source, dest);

  for entry in WalkDir::new(source).sort(true) {
    let entry = entry?;
    let entry_path_abs = entry.path();
    let entry_path_rel = entry_path_abs.strip_prefix(source)?;
    let dest_path_abs = dest.join(entry_path_rel);

    if entry_path_rel == PathBuf::from("") {
      continue;
    }

    if dest_path_abs.exists() {
      return Err(anyhow::anyhow!("PathAlreadyExists: {:?}", dest_path_abs));
    }

    let Ok(entry_meta) = fs::metadata(&entry_path_abs) else {
      log::info!("copy:err:skipping: {:?}", entry_path_rel);
      continue;
    };
    if entry_meta.is_dir() {
      fs::create_dir_all(dest_path_abs)?;
    } else {
      hard_link_or_copy(entry_path_abs, dest_path_abs)?;
    }
  }

  Ok(())
}

/// Remove a file or folder if it exists
pub fn remove_if_exists<P: AsRef<Path>>(target: P) -> std::io::Result<()> {
  let target = target.as_ref();

  if !fs::exists(target)? {
    log::info!("remv:does_not_exist: {:?}", target);
    return Ok(());
  }

  let meta = fs::metadata(target)?;

  if meta.is_dir() {
    log::info!("remv:dir: {:?}", target);
    return fs::remove_dir_all(target);
  }

  log::info!("remv:file: {:?}", target);
  fs::remove_file(target)
}

/// Create a directory and delete and existing if it already exists
pub fn recreate_dir<P: AsRef<Path>>(target: P) -> std::io::Result<()> {
  let target = target.as_ref();
  remove_if_exists(target)?;
  log::info!("creating_dir: {:?}", target);
  fs::create_dir_all(target)
}

/// Creates a directory if it doesn't exist
pub fn create_dir_if_not_exists<P: AsRef<Path>>(target: P) -> std::io::Result<()> {
  let target = target.as_ref();
  if fs::exists(target)? {
    log::info!("create_dir:already_exists: {:?}", target);
    return Ok(());
  }
  log::info!("create_dir:create: {:?}", target);
  fs::create_dir_all(target)
}

pub fn append_ext(ext: impl AsRef<OsStr>, path: impl AsRef<Path>) -> PathBuf {
  let mut os_string: OsString = path.as_ref().into();
  os_string.push(".");
  os_string.push(ext.as_ref());
  os_string.into()
}
