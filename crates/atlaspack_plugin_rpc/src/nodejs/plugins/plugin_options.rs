use std::path::PathBuf;

use atlaspack_core::{plugin::HmrOptions, types::BuildMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcPluginOptions {
  pub hmr_options: Option<HmrOptions>,
  pub project_root: PathBuf,
  pub mode: BuildMode,
}
