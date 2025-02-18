use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
  pub file_path: PathBuf,
  pub target: Option<String>,
}
