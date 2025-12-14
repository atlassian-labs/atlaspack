use std::path::PathBuf;

use atlaspack_core::{
  plugin::HmrOptions,
  types::{BuildMode, FeatureFlags},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcPluginOptions {
  pub hmr_options: Option<HmrOptions>,
  pub project_root: PathBuf,
  pub mode: BuildMode,
  pub feature_flags: FeatureFlags,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadPluginKind {
  Resolver,
  Transformer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
  pub options: RpcPluginOptions,
}
