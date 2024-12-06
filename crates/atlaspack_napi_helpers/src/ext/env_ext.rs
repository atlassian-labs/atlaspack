use napi::Env;
use napi::JsUndefined;
use napi::NapiValue;

use crate::console_log;
use crate::JsResolvable;

pub trait NapiEnvExt {
  /// Create Promise-like object that can be resolved
  /// from threads it wasn't created on
  fn create_resolvable(&self) -> napi::Result<JsResolvable>;
  /// Runs console.log() in the JavaScript context.
  /// useful for debugging [`NapiValue`] types
  fn console_log<V: NapiValue>(&self, args: &[V]) -> napi::Result<JsUndefined>;
}

impl NapiEnvExt for Env {
  fn create_resolvable(&self) -> napi::Result<JsResolvable> {
    JsResolvable::new(self)
  }

  fn console_log<V: NapiValue>(&self, args: &[V]) -> napi::Result<JsUndefined> {
    console_log(self, args)
  }
}
