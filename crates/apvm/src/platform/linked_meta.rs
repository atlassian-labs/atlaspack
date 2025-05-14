use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::package_kind::PackageKind;
use super::package_path::PackagePath;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedMeta {
  pub kind: PackageKind,
  pub specifier: Option<String>,
  pub original_path: PackagePath,
}

impl LinkedMeta {
  pub fn read_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    Ok(serde_json::from_slice::<Self>(&std::fs::read(path)?)?)
  }

  pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    Ok(fs::write(path, serde_json::to_string_pretty(self)?)?)
  }
}
