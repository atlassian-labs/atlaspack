use napi::bindgen_prelude::ToNapiValue;
use napi::Env;
use napi::JsObject;

use crate::TupleResult;

/// Converts return type to JavaScript Tuple representing Rust's Result type
/// ```
/// [JsAny, null]
/// ```
pub fn map_to_ok_tuple<JsRet: ToNapiValue + 'static>(
  env: Env,
  ret: JsRet,
) -> napi::Result<JsObject> {
  TupleResult::ok(&env, ret)
}
