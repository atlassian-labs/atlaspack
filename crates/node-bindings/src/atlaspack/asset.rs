use atlaspack_core::types::Environment;
use napi::{Env, JsUnknown};
use napi_derive::napi;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIdParams {
  env: Environment,
  file_path: String,
  code: Option<String>,
  pipeline: Option<String>,
  query: Option<String>,
  unique_key: Option<String>,
}

#[napi]
pub fn create_asset_id(env: Env, params: JsUnknown) -> napi::Result<String> {
  let AssetIdParams {
    env,
    file_path,
    code,
    pipeline,
    query,
    unique_key,
  } = env.from_js_value(params)?;

  let asset_id =
    atlaspack_core::types::create_asset_id(atlaspack_core::types::CreateAssetIdParams {
      env: &env,
      file_path: &file_path,
      code: code.as_deref(),
      pipeline: pipeline.as_deref(),
      query: query.as_deref(),
      unique_key: unique_key.as_deref(),
    });
  Ok(asset_id)
}
