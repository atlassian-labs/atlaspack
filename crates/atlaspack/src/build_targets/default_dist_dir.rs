use std::path::Path;
use std::path::PathBuf;

pub fn default_dist_dir(package_path: &Path) -> PathBuf {
  package_path.parent().unwrap_or(package_path).join("dist")
}
