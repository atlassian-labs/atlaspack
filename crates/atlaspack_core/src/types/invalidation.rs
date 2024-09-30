use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Invalidation {
  FileChange(PathBuf),
}
