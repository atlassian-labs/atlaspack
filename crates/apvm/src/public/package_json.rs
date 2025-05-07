use serde::Deserialize;
use serde::Serialize;

use super::json_serde::JsonSerde;

impl JsonSerde for PackageJson {}

#[allow(unused)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PackageJson {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub private: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub r#type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub main: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub types: Option<String>,
}
