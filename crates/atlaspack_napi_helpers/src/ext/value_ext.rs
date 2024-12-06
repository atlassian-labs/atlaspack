use napi::Env;
use napi::NapiValue;

use crate::JsArc;

pub trait NapiRawExt<T: NapiValue> {
  fn into_arc(self, env: &Env) -> napi::Result<JsArc<T>>;
}

impl<T: NapiValue> NapiRawExt<T> for T {
  fn into_arc(self, env: &Env) -> napi::Result<JsArc<T>> {
    JsArc::new(env, self)
  }
}
