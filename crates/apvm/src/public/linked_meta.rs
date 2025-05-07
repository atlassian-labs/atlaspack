use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::json_serde::JsonSerde;
use super::package_kind::PackageKind;

impl JsonSerde for LinkedMeta {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedMeta {
  pub specifier: Option<String>,
  pub kind: PackageKind,
  pub version: Option<String>,
  pub original_path: PathBuf,
  pub content_path: PathBuf,
}
