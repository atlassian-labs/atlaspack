use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadPluginKind {
  Resolver,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
}
