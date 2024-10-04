#![allow(unused)]

use atlaspack_napi_helpers::js_callable::map_params_serde;
use atlaspack_napi_helpers::js_callable::map_return_serde;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::bindgen_prelude::Buffer;
use napi::bindgen_prelude::FromNapiValue;
use napi::Env;
use napi::JsFunction;
use napi::JsObject;
use napi::JsString;
use napi::JsUndefined;
use napi::JsUnknown;
use napi::ValueType;
use napi_derive::napi;

#[derive(Clone)]
#[napi]
pub struct FsBridge {
  read_file_fn: JsCallable,
  read_file_sync_fn: JsCallable,
  write_file_fn: JsCallable,
  copy_file_fn: JsCallable,
  stat_fn: JsCallable,
  stat_sync_fn: JsCallable,
  readdir_fn: JsCallable,
  readdir_sync_fn: JsCallable,
  symlink_fn: JsCallable,
  unlink_fn: JsCallable,
  realpath_fn: JsCallable,
  realpath_sync_fn: JsCallable,
  exists_fn: JsCallable,
  exists_sync_fn: JsCallable,
  mkdirp_fn: JsCallable,
  rimraf_fn: JsCallable,
  ncp_fn: JsCallable,
  create_read_stream_fn: JsCallable,
  create_write_stream_fn: JsCallable,
  cwd_fn: JsCallable,
  chdir_fn: JsCallable,
  watch_fn: JsCallable,
  get_events_since_fn: JsCallable,
  write_snapshot_fn: JsCallable,
  find_ancestor_file_fn: JsCallable,
  find_node_module_fn: JsCallable,
  find_first_file_fn: JsCallable,
}

#[napi]
impl FsBridge {
  #[napi(constructor)]
  pub fn new(target: JsObject) -> napi::Result<Self> {
    Ok(FsBridge {
      read_file_fn: JsCallable::new_from_object_prop_bound("readFile", &target)?,
      read_file_sync_fn: JsCallable::new_from_object_prop_bound("readFileSync", &target)?,
      write_file_fn: JsCallable::new_from_object_prop_bound("writeFile", &target)?,
      copy_file_fn: JsCallable::new_from_object_prop_bound("copyFile", &target)?,
      stat_fn: JsCallable::new_from_object_prop_bound("stat", &target)?,
      stat_sync_fn: JsCallable::new_from_object_prop_bound("statSync", &target)?,
      readdir_fn: JsCallable::new_from_object_prop_bound("readdir", &target)?,
      readdir_sync_fn: JsCallable::new_from_object_prop_bound("readdirSync", &target)?,
      symlink_fn: JsCallable::new_from_object_prop_bound("symlink", &target)?,
      unlink_fn: JsCallable::new_from_object_prop_bound("unlink", &target)?,
      realpath_fn: JsCallable::new_from_object_prop_bound("realpath", &target)?,
      realpath_sync_fn: JsCallable::new_from_object_prop_bound("realpathSync", &target)?,
      exists_fn: JsCallable::new_from_object_prop_bound("exists", &target)?,
      exists_sync_fn: JsCallable::new_from_object_prop_bound("existsSync", &target)?,
      mkdirp_fn: JsCallable::new_from_object_prop_bound("mkdirp", &target)?,
      rimraf_fn: JsCallable::new_from_object_prop_bound("rimraf", &target)?,
      ncp_fn: JsCallable::new_from_object_prop_bound("ncp", &target)?,
      create_read_stream_fn: JsCallable::new_from_object_prop_bound("createReadStream", &target)?,
      create_write_stream_fn: JsCallable::new_from_object_prop_bound("createWriteStream", &target)?,
      cwd_fn: JsCallable::new_from_object_prop_bound("cwd", &target)?,
      chdir_fn: JsCallable::new_from_object_prop_bound("chdir", &target)?,
      watch_fn: JsCallable::new_from_object_prop_bound("watch", &target)?,
      get_events_since_fn: JsCallable::new_from_object_prop_bound("getEventsSince", &target)?,
      write_snapshot_fn: JsCallable::new_from_object_prop_bound("writeSnapshot", &target)?,
      find_ancestor_file_fn: JsCallable::new_from_object_prop_bound("findAncestorFile", &target)?,
      find_node_module_fn: JsCallable::new_from_object_prop_bound("findNodeModule", &target)?,
      find_first_file_fn: JsCallable::new_from_object_prop_bound("findFirstFile", &target)?,
    })
  }

  #[napi]
  pub fn read_file(
    &self,
    env: Env,
    file_path: String,
    encoding: Option<String>,
  ) -> napi::Result<JsUnknown> {
    if let Some(encoding) = encoding {
      let result: String = self
        .read_file_fn
        .call_with_return(map_params_serde((file_path, encoding)), map_return_serde())?;
      return Ok(env.create_string(&result)?.into_unknown());
    } else {
      let result: Vec<u8> = self
        .read_file_fn
        .call_with_return(map_params_serde((file_path)), |_env, buf| {
          Ok(Buffer::from_unknown(buf)?.into())
        })?;
      return env.to_js_value(&result);
    }
  }

  #[napi]
  pub fn read_file_sync(
    &self,
    env: Env,
    file_path: String,
    encoding: Option<String>,
  ) -> napi::Result<JsUnknown> {
    if let Some(encoding) = encoding {
      let result: String = self
        .read_file_sync_fn
        .call_with_return(map_params_serde((file_path, encoding)), map_return_serde())?;
      return Ok(env.create_string(&result)?.into_unknown());
    } else {
      let result: Vec<u8> = self
        .read_file_sync_fn
        .call_with_return(map_params_serde((file_path)), |_env, buf| {
          Ok(Buffer::from_unknown(buf)?.into())
        })?;
      return env.to_js_value(&result);
    }
  }

  #[napi]
  pub fn write_file(
    &self,
    env: Env,
    file_path: String,
    contents: JsUnknown,
    options: Option<JsObject>,
  ) -> napi::Result<JsUndefined> {
    if contents.get_type()? == ValueType::String {
      let contents = JsString::from_unknown(contents)?;
      let contents = contents.into_utf8()?.into_owned()?;

      self
        .write_file_fn
        .call_with_return(map_params_serde((file_path, contents)), |_, _| Ok(()))?;
    } else {
      let contents = Buffer::from_unknown(contents)?;
      let contents: Vec<u8> = contents.into();

      self
        .write_file_fn
        .call_with_return(map_params_serde((file_path, contents)), |_, _| Ok(()))?;
    }

    env.get_undefined()
  }

  #[napi]
  pub fn copy_file(
    &self,
    env: Env,
    source: String,
    destination: String,
    flags: Option<i32>,
  ) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn stat(&self, env: Env, file_path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn stat_sync(&self, env: Env, file_path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn readdir(&self, env: Env, file_path: String, options: JsObject) -> napi::Result<JsUnknown> {
    unimplemented!()
  }

  #[napi]
  pub fn readdir_sync(
    &self,
    env: Env,
    file_path: String,
    options: JsObject,
  ) -> napi::Result<JsUnknown> {
    unimplemented!()
  }

  #[napi]
  pub fn symlink(&self, env: Env, target: String, path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn unlink(&self, env: Env, path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn realpath(&self, env: Env, path: String) -> napi::Result<String> {
    unimplemented!()
  }

  #[napi]
  pub fn realpath_sync(&self, env: Env, path: String) -> napi::Result<String> {
    unimplemented!()
  }

  #[napi]
  pub fn exists(&self, env: Env, path: String) -> napi::Result<bool> {
    unimplemented!()
  }

  #[napi]
  pub fn exists_sync(&self, env: Env, path: String) -> napi::Result<bool> {
    unimplemented!()
  }

  #[napi]
  pub fn mkdirp(&self, env: Env, path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn rimraf(&self, env: Env, path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn ncp(&self, env: Env, source: String, destination: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn create_read_stream(
    &self,
    env: Env,
    path: String,
    options: Option<JsObject>,
  ) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn create_write_stream(
    &self,
    env: Env,
    path: String,
    options: Option<JsObject>,
  ) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn cwd(&self, env: Env) -> napi::Result<String> {
    unimplemented!()
  }

  #[napi]
  pub fn chdir(&self, env: Env, path: String) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn watch(
    &self,
    env: Env,
    dir: String,
    func: JsFunction,
    opts: JsObject,
  ) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn get_events_since(
    &self,
    env: Env,
    path: String,
    opts: JsObject,
  ) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn write_snapshot(
    &self,
    env: Env,
    path: String,
    opts: JsObject,
  ) -> napi::Result<JsUndefined> {
    unimplemented!()
  }

  #[napi]
  pub fn find_ancestor_file(&self, env: Env) -> napi::Result<Option<String>> {
    unimplemented!()
  }

  #[napi]
  pub fn find_node_module(&self, env: Env) -> napi::Result<Option<String>> {
    unimplemented!()
  }

  #[napi]
  pub fn find_first_file(&self, env: Env) -> napi::Result<Option<String>> {
    unimplemented!()
  }
}
