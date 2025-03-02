use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::types::Environment;

#[derive(Hash)]
pub struct AssetHash {
  project_root: PathBuf,
  code: Option<String>,
  env: Arc<Environment>,
  file_path: PathBuf,
  pipeline: Option<String>,
  query: Option<String>,
  side_effects: bool,
}

impl AssetHash {
  pub fn calculate(
    project_root: PathBuf,
    code: Option<String>,
    env: Arc<Environment>,
    file_path: PathBuf,
    pipeline: Option<String>,
    query: Option<String>,
    side_effects: bool,
  ) -> u64 {
    let mut hasher = atlaspack_core::hash::IdentifierHasher::default();
    Self {
      project_root,
      code,
      env,
      file_path,
      pipeline,
      query,
      side_effects,
    }
    .hash(&mut hasher);
    hasher.finish()
  }
}
