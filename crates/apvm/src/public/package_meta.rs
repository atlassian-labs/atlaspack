use serde::Deserialize;
use serde::Serialize;

use super::json_serde::JsonSerde;
use super::package_kind::PackageKind;

impl JsonSerde for PackageMeta {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageMeta {
  pub specifier: Option<String>,
  pub kind: PackageKind,
  pub version: Option<String>,
  pub checksum: Option<String>,
}
