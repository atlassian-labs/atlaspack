use napi::Env;
use napi::JsObject;
use serde::Serialize;

use crate::TupleResult;

/// Converts return type to JavaScript Tuple representing Rust's Result type
/// ```
/// [JsAny, null]
/// ```
pub fn map_to_ok_tuple_serde<JsRet: Serialize>(env: Env, ret: JsRet) -> napi::Result<JsObject> {
  let result = env.to_js_value(&ret)?;
  TupleResult::ok(&env, result)
}
