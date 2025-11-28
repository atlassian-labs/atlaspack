use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use mockall::automock;
use serde::Deserialize;

/// PackageManager abstraction instance
pub type PackageManagerRef = Arc<dyn PackageManager + Send + Sync>;

#[derive(Debug, Deserialize)]
pub struct Resolution {
  pub resolved: PathBuf,
}

#[derive(Debug)]
pub struct DevDep {
  pub file_path: PathBuf,
  pub hash: Option<u64>,
}

#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
#[automock]
pub trait PackageManager {
  fn resolve(&self, specifier: &str, from: &Path) -> anyhow::Result<Resolution>;
  fn resolve_dev_dependency(
    &self,
    package_name: &str,
    resolve_from: &Path,
  ) -> anyhow::Result<DevDep>;
}
