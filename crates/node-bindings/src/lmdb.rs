use napi::bindgen_prelude::Buffer;
use napi::bindgen_prelude::Env;
use napi::JsUnknown;
use napi_derive::napi;
#[napi]
pub struct LMDB {
  inner: lmdb_js_lite::LMDB,
}

#[napi]
impl LMDB {
  #[napi(constructor)]
  pub fn new(options: lmdb_js_lite::LMDBOptions) -> napi::Result<Self> {
    Ok(Self {
      inner: lmdb_js_lite::LMDB::new(options)?,
    })
  }

  #[napi(ts_return_type = "Promise<Buffer | null | undefined>")]
  pub fn get(&self, env: Env, key: String) -> napi::Result<napi::JsObject> {
    self.inner.get(env, key)
  }

  #[napi(ts_return_type = "Buffer | null")]
  pub fn get_sync(&self, env: Env, key: String) -> napi::Result<JsUnknown> {
    self.inner.get_sync(env, key)
  }

  #[napi]
  pub fn get_many_sync(&self, keys: Vec<String>) -> napi::Result<Vec<Option<Buffer>>> {
    self.inner.get_many_sync(keys)
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn put_many(
    &self,
    env: Env,
    entries: Vec<lmdb_js_lite::Entry>,
  ) -> napi::Result<napi::JsObject> {
    self.inner.put_many(env, entries)
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn put(&self, env: Env, key: String, data: Buffer) -> napi::Result<napi::JsObject> {
    self.inner.put(env, key, data)
  }

  #[napi]
  pub fn put_no_confirm(&self, key: String, data: Buffer) -> napi::Result<()> {
    self.inner.put_no_confirm(key, data)
  }

  #[napi]
  pub fn start_read_transaction(&mut self) -> napi::Result<()> {
    self.inner.start_read_transaction()
  }

  #[napi]
  pub fn commit_read_transaction(&mut self) -> napi::Result<()> {
    self.inner.commit_read_transaction()
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn start_write_transaction(&self, env: Env) -> napi::Result<napi::JsObject> {
    self.inner.start_write_transaction(env)
  }

  #[napi(ts_return_type = "Promise<void>")]
  pub fn commit_write_transaction(&self, env: Env) -> napi::Result<napi::JsObject> {
    self.inner.commit_write_transaction(env)
  }

  #[napi]
  pub fn close(&mut self) {
    self.inner.close()
  }
}
