use napi::bindgen_prelude::ToNapiValue;
use napi::Env;
use napi::JsObject;

/// This creates the following JavaScript tuple
/// ```
/// type Ok<T> = [T, null]
/// type Err<E> = [null, E]
/// type Result<T, E> = Ok<T> | Err<E>
/// ```
pub struct JsResult;

impl JsResult {
  /// This creates the following JavaScript tuple
  /// ```
  /// [JsAny, null]
  /// ```
  pub fn ok(env: &Env, target: impl ToNapiValue) -> napi::Result<JsObject> {
    let mut obj = env.create_array(2)?;
    obj.set(0, target)?;
    obj.set(1, env.get_null())?;
    obj.coerce_to_object()
  }

  /// This creates the following JavaScript tuple
  /// ```
  /// [null, JsAny]
  /// ```
  pub fn error(env: &Env, target: impl ToNapiValue) -> napi::Result<JsObject> {
    let mut obj = env.create_array(2)?;
    obj.set(0, env.get_null())?;
    obj.set(1, target)?;
    obj.coerce_to_object()
  }
}
