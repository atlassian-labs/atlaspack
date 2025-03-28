use std::thread;

use napi::bindgen_prelude::FromNapiValue;
use napi::bindgen_prelude::ToNapiValue;
use napi::Env;
use napi::JsObject;
use napi::JsUnknown;

/// Spawns an OS thread and returns a Promise
pub fn spawn<Ret: 'static, JsRet: ToNapiValue + 'static, JsErr: ToNapiValue + 'static>(
  env: &Env,
  callback: impl 'static + Send + FnOnce() -> anyhow::Result<Ret>,
  map_result: impl 'static + Send + FnOnce(Env, Ret) -> napi::Result<JsRet>,
  map_error: impl 'static + Send + FnOnce(Env, anyhow::Error) -> napi::Result<JsErr>,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;

  thread::spawn(move || {
    // Run callback on separate thread
    let result = callback();

    deferred.resolve(move |env| match result {
      Ok(result) => {
        let result = map_result(env, result)?;
        // Safety: Type must be convertible to JsUnknown because of ToNapiValue trait
        unsafe { JsUnknown::from_napi_value(env.raw(), JsRet::to_napi_value(env.raw(), result)?) }
      }
      Err(error) => {
        let result = map_error(env, error)?;
        // Safety: Type must be convertible to JsUnknown because of ToNapiValue trait
        unsafe { JsUnknown::from_napi_value(env.raw(), JsErr::to_napi_value(env.raw(), result)?) }
      }
    });
  });

  Ok(promise)
}
