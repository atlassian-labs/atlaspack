use std::thread;

use atlaspack::AtlaspackError;
use napi::bindgen_prelude::ToNapiValue;
use napi::Env;
use napi::JsObject;
use napi::JsUnknown;

pub trait NapiExt {
  fn tuple_ok(&self, value: impl ToNapiValue) -> napi::Result<JsObject>;
  fn tuple_err(&self, error: impl ToNapiValue) -> napi::Result<JsObject>;
  fn atlaspack_err(&self, error: &anyhow::Error) -> napi::Result<JsUnknown>;
  fn spawn_thread<ThreadFunc, NapiFunc, NapiRet>(&self, func: ThreadFunc) -> napi::Result<JsObject>
  where
    ThreadFunc: FnOnce() -> napi::Result<NapiFunc> + Send + 'static,
    NapiFunc: FnOnce(Env) -> napi::Result<NapiRet> + Send + 'static,
    NapiRet: ToNapiValue;
}

impl NapiExt for Env {
  /// This creates the following JavaScript tuple
  /// ```
  /// [JsAny, null]
  /// ```
  fn tuple_ok(&self, value: impl ToNapiValue) -> napi::Result<JsObject> {
    let mut obj = self.create_array(2)?;
    obj.set(0, value)?;
    obj.set(1, self.get_null())?;
    obj.coerce_to_object()
  }

  /// This creates the following JavaScript tuple
  /// ```
  /// [null, JsAny]
  /// ```
  fn tuple_err(&self, error: impl ToNapiValue) -> napi::Result<JsObject> {
    let mut obj = self.create_array(2)?;
    obj.set(0, self.get_null())?;
    obj.set(1, error)?;
    obj.coerce_to_object()
  }

  /// This casts an error to a string and returns it as a JavaScript tuple
  fn atlaspack_err(&self, error: &anyhow::Error) -> napi::Result<JsUnknown> {
    self.to_js_value(&AtlaspackError::from(error))
  }

  fn spawn_thread<ThreadFunc, NapiFunc, NapiRet>(&self, func: ThreadFunc) -> napi::Result<JsObject>
  where
    ThreadFunc: FnOnce() -> napi::Result<NapiFunc> + Send + 'static,
    NapiFunc: FnOnce(Env) -> napi::Result<NapiRet> + Send + 'static,
    NapiRet: ToNapiValue,
  {
    let (deferred, promise) = self.create_deferred()?;

    thread::spawn(move || match func() {
      Ok(napi_func) => {
        deferred.resolve(|env: Env| match napi_func(env) {
          Ok(result) => Ok(env.tuple_ok(result)),
          Err(error) => Ok(env.tuple_err(error)),
        });
      }
      Err(error) => deferred.reject(napi::Error::from_reason(format!("{:?}", error))),
    });

    Ok(promise)
  }
}

pub type MapJsParams = Box<dyn Send + FnOnce(&Env) -> anyhow::Result<Vec<JsUnknown>> + 'static>;
