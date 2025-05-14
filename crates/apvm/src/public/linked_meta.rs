use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::json_serde::JsonSerde;
use super::package_kind::PackageKind;

impl JsonSerde for LinkedMeta {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedMeta {
  pub kind: PackageKind,
  pub specifier: Option<String>,
  pub original_path: PathBuf,
}
