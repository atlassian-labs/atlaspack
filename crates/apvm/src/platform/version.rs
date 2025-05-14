use serde::{Deserialize, Serialize};

use super::{package_kind::PackageKind, package_path::PackagePath};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
  pub kind: PackageKind,
  pub specifier: Option<String>,
  pub path: PackagePath,
}
