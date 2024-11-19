use std::{
  io,
  path::{Path, PathBuf},
};

use atlaspack::file_system::FileSystem;
use napi::{Env, JsObject};

use atlaspack_napi_helpers::js_callable::JsCallable;

// TODO error handling

#[derive(Debug)]
pub struct FileSystemNapi {
  canonicalize_fn: JsCallable,
  create_directory_fn: JsCallable,
  cwd_fn: JsCallable,
  read_file_fn: JsCallable,
  is_file_fn: JsCallable,
  is_dir_fn: JsCallable,
}

impl FileSystemNapi {
  pub fn new(env: &Env, js_file_system: &JsObject) -> napi::Result<Self> {
    Ok(Self {
      canonicalize_fn: JsCallable::new_method("canonicalize", js_file_system)?.into_unref(env)?,
      create_directory_fn: JsCallable::new_method("createDirectory", js_file_system)?
        .into_unref(env)?,
      cwd_fn: JsCallable::new_method("cwd", js_file_system)?.into_unref(env)?,
      read_file_fn: JsCallable::new_method("readFile", js_file_system)?.into_unref(env)?,
      is_file_fn: JsCallable::new_method("isFile", js_file_system)?.into_unref(env)?,
      is_dir_fn: JsCallable::new_method("isDir", js_file_system)?.into_unref(env)?,
    })
  }
}

impl FileSystem for FileSystemNapi {
  fn canonicalize_base(&self, path: &Path) -> io::Result<PathBuf> {
    self
      .canonicalize_fn
      .call_serde_blocking(path.to_path_buf())
      .map_err(io::Error::other)
  }

  fn create_directory(&self, path: &Path) -> std::io::Result<()> {
    self
      .create_directory_fn
      .call_serde_blocking(path.to_path_buf())
      .map_err(io::Error::other)
  }

  fn cwd(&self) -> io::Result<PathBuf> {
    self
      .cwd_fn
      .call_serde_blocking(None::<bool>)
      .map_err(io::Error::other)
  }

  fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
    self
      .read_file_fn
      .call_serde_blocking(path.to_path_buf())
      .map_err(io::Error::other)
  }

  fn read_to_string(&self, path: &Path) -> io::Result<String> {
    self
      .read_file_fn
      .call_serde_blocking((path.to_path_buf(), "utf8"))
      .map_err(io::Error::other)
  }

  fn is_file(&self, path: &Path) -> bool {
    self
      .is_file_fn
      .call_serde_blocking(path.to_path_buf())
      .expect("TODO handle error case")
  }

  fn is_dir(&self, path: &Path) -> bool {
    self
      .is_dir_fn
      .call_serde_blocking(path.to_path_buf())
      .expect("TODO handle error case")
  }

  fn read_dir(&self, _path: &Path) -> std::io::Result<std::fs::ReadDir> {
    todo!("InMemoryFileSystem::read_dir")
  }
}
