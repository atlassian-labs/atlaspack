use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::types::Location;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintableError {
  pub file_name: Option<String>,
  pub file_path: Option<String>,
  pub code_frame: Option<String>,
  pub highlighted_code_frame: Option<String>,
  pub loc: Option<Location>,
  pub source: Option<String>,
}

impl Display for PrintableError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}
