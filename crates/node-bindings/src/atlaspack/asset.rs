use atlaspack_core::types::FileType;
use napi::{Env, JsUnknown};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIdParams {
  code: Option<String>,
  environment_id: String,
  file_path: String,
  file_type: FileType,
  pipeline: Option<String>,
  query: Option<String>,
  unique_key: Option<String>,
}

#[napi]
pub fn create_asset_id(env: Env, params: JsUnknown) -> napi::Result<String> {
  let AssetIdParams {
    code,
    environment_id,
    file_path,
    file_type,
    pipeline,
    query,
    unique_key,
  } = env.from_js_value(params)?;

  let asset_id =
    atlaspack_core::types::create_asset_id(atlaspack_core::types::CreateAssetIdParams {
      code: code.as_deref(),
      environment_id: &environment_id,
      file_path: &file_path,
      file_type: &file_type,
      pipeline: pipeline.as_deref(),
      query: query.as_deref(),
      unique_key: unique_key.as_deref(),
    });

  Ok(asset_id)
}
