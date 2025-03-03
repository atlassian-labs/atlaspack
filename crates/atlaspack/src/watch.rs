use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchEventType {
  Create,
  Update,
  Delete,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchEvent {
  pub path: PathBuf,
  pub kind: WatchEventType,
}

pub type WatchEvents = Vec<WatchEvent>;
