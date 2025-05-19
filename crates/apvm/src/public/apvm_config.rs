use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use super::json_serde::JsonSerde;

impl JsonSerde for ApvmConfig {}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct ApvmConfig {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub versions: Option<HashMap<String, String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub checksums: Option<HashMap<String, String>>,
}
