use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "path", rename_all = "camelCase")]
pub enum WatchEvent {
  Create(PathBuf),
  Update(PathBuf),
  Delete(PathBuf),
}

pub type WatchEvents = Vec<WatchEvent>;
