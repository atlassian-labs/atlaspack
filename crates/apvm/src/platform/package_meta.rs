use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use super::package_kind::PackageKind;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageMeta {
  pub kind: PackageKind,
  /// Will be None if the PackageKind is unmanaged or local
  pub specifier: Option<String>,
}

impl PackageMeta {
  pub fn read_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    Ok(serde_json::from_slice::<Self>(&std::fs::read(path)?)?)
  }

  pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    Ok(fs::write(path, serde_json::to_string_pretty(self)?)?)
  }
}
