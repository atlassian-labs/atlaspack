use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::json_serde::JsonSerde;

impl JsonSerde for PackageJson {}

#[allow(unused)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PackageJson {
  pub name: Option<String>,
  pub version: Option<String>,
  pub private: Option<bool>,
  pub atlaspack: Option<PackageJsonAtlaspack>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PackageJsonAtlaspack {
  pub version: Option<String>,
  pub versions: Option<HashMap<String, String>>,
}
