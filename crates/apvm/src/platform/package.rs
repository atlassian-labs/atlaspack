use std::path::{Path, PathBuf};

use crate::platform::constants as c;
use serde::{Deserialize, Serialize};

use super::encoder;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Package {
  Local(LocalPackage),
  Npm(NpmPackage),
  Git(GitPackage),
  Unmanaged(UnmanagedPackage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManagedPackage {
  Local(LocalPackage),
  Npm(NpmPackage),
  Git(GitPackage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallablePackage {
  Npm(NpmPackage),
  Git(GitPackage),
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

impl NpmPackage {
  pub fn from_name(base: &Path, version: &str) -> anyhow::Result<Self> {
    Ok(Self {
      version: version.to_string(),
      path: base.join(encoder::encode(version)?),
    })
  }

  pub fn meta(&self) -> PathBuf {
    self.path.join(c::PACKAGE_META_FILE)
  }

  pub fn contents(&self) -> PathBuf {
    self.path.join("contents")
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GitPackage {
  pub branch: String,
  /// $APVM_DIR/versions_v1/<name>
  pub path: PathBuf,
}

impl GitPackage {
  pub fn from_name(base: &Path, branch: &str) -> anyhow::Result<Self> {
    Ok(Self {
      branch: branch.to_string(),
      path: base.join(encoder::encode(branch)?),
    })
  }

  pub fn meta(&self) -> PathBuf {
    self.path.join(c::PACKAGE_META_FILE)
  }

  pub fn contents(&self) -> PathBuf {
    self.path.join("contents")
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UnmanagedPackage {
  /// $PWD/node_modules/@atlaspack
  pub path: PathBuf,
}
