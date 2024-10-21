use std::path::PathBuf;

use atlaspack_core::types::BuildMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcHmrOptions {
  pub port: Option<u16>,
  pub host: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcPluginOptions {
  pub hmr_options: Option<RpcHmrOptions>,
  pub project_root: PathBuf,
  pub mode: BuildMode,
}
