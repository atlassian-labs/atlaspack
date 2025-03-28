use atlaspack_core::error::AtlaspackError;
use napi::Env;
use napi::JsObject;

use crate::NapiResult;

/// Converts return type to JavaScript Tuple representing Rust's Result type
/// ```
/// [null, JsAny]
/// ```
pub fn map_to_err_tuple(env: Env, error: anyhow::Error) -> napi::Result<JsObject> {
  let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
  NapiResult::error(&env, js_object)
}
