use std::{
  io,
  path::{Path, PathBuf},
};

use atlaspack::file_system::{FileSystem, Metadata};
use napi::{Env, JsObject};

use atlaspack_napi_helpers::js_callable::JsCallable;

// TODO error handling

pub struct FileSystemNapi {
  canonicalize_fn: JsCallable,
  create_dir_all_fn: JsCallable,
  cwd_fn: JsCallable,
  read_to_string_fn: JsCallable,
  read_fn: JsCallable,
  exists_fn: JsCallable,
  is_file_fn: JsCallable,
  is_dir_fn: JsCallable,
}

impl FileSystemNapi {
  pub fn new(env: &Env, js_file_system: &JsObject) -> napi::Result<Self> {
    Ok(Self {
      canonicalize_fn: JsCallable::new_from_object_prop("canonicalize", &js_file_system)?
        .into_unref(env)?,
      create_dir_all_fn: JsCallable::new_from_object_prop("createDirAll", &js_file_system)?
        .into_unref(env)?,
      cwd_fn: JsCallable::new_from_object_prop("cwd", &js_file_system)?.into_unref(env)?,
      read_to_string_fn: JsCallable::new_from_object_prop("readToString", &js_file_system)?
        .into_unref(env)?,
      exists_fn: JsCallable::new_from_object_prop("exists", &js_file_system)?.into_unref(env)?,
      read_fn: JsCallable::new_from_object_prop("read", &js_file_system)?.into_unref(env)?,
      is_file_fn: JsCallable::new_from_object_prop("isFile", &js_file_system)?.into_unref(env)?,
      is_dir_fn: JsCallable::new_from_object_prop("isDir", &js_file_system)?.into_unref(env)?,
    })
  }
}

impl FileSystem for FileSystemNapi {
  fn cwd(&self) -> io::Result<PathBuf> {
    self
      .cwd_fn
      .call_with_return_serde(None::<bool>)
      .map_err(|e| io::Error::other(e))
  }

  fn exists(&self, path: &Path) -> io::Result<bool> {
    self
      .exists_fn
      .call_with_return_serde(path.to_path_buf())
      .map_err(|e| io::Error::other(e))
  }

  fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
    self
      .canonicalize_fn
      .call_with_return_serde(path.to_path_buf())
      .map_err(|e| io::Error::other(e))
  }

  fn create_dir_all(&self, path: &Path) -> io::Result<()> {
    self
      .create_dir_all_fn
      .call_with_return_serde(path.to_path_buf())
      .map_err(|e| io::Error::other(e))
  }

  fn read_to_string(&self, path: &Path) -> io::Result<String> {
    self
      .read_to_string_fn
      .call_with_return_serde((path.to_path_buf(), "utf8"))
      .map_err(|e| io::Error::other(e))
  }

  fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
    self
      .read_to_string_fn
      .call_with_return_serde((path.to_path_buf(), "buffer"))
      .map_err(|e| io::Error::other(e))
  }

  fn metadata(&self, path: &Path) -> io::Result<Box<dyn Metadata>> {
    Ok(Box::new(FileSystemMetadataNapi {
      path: path.to_path_buf(),
      is_dir_fn: self.is_dir_fn.clone(),
      is_file_fn: self.is_file_fn.clone(),
    }))
  }
}

pub struct FileSystemMetadataNapi {
  path: PathBuf,
  is_dir_fn: JsCallable,
  is_file_fn: JsCallable,
}

impl Metadata for FileSystemMetadataNapi {
  fn is_dir(&self) -> bool {
    self
      .is_dir_fn
      .call_with_return_serde(self.path.clone())
      .expect("Cannot get metadata for file")
  }

  fn is_file(&self) -> bool {
    self
      .is_file_fn
      .call_with_return_serde(self.path.clone())
      .expect("Cannot get metadata for file")
  }
}
