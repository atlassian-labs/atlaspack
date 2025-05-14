use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Package {
  Local(LocalPackage),
  Npm(NpmPackage),
  Git(GitPackage),
  Unmanaged(UnmanagedPackage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalPackage {
  /// $APVM_ATLASPACK_LOCAL
  pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NpmPackage {
  pub version: String,
  /// $APVM_DIR/versions_v1/<name>
  pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GitPackage {
  pub branch: String,
  /// $APVM_DIR/versions_v1/<name>
  pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UnmanagedPackage {
  /// $PWD/node_modules/@atlaspack
  pub path: PathBuf,
}
