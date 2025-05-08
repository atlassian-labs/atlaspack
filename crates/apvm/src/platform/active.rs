use std::fs;
use std::path::PathBuf;

use super::constants as c;
use super::origin::VersionTarget;
use super::package::PackageDescriptor;
use crate::paths::Paths;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum ActiveType {
  SystemDefault,
  NodeModules,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ActiveVersion {
  pub active_type: ActiveType,
  pub package: PackageDescriptor,
  pub node_modules_path: Option<PathBuf>,
}

impl ActiveVersion {
  pub fn detect(paths: &Paths) -> anyhow::Result<Option<Self>> {
    // Detect from node_modules
    if let Some(atlaspack_version_path) = &paths.node_modules_atlaspack {
      let version_path = atlaspack_version_path.join(c::APVM_VERSION_FILE);
      if fs::exists(&version_path)? {
        let version = fs::read_to_string(atlaspack_version_path.join(c::APVM_VERSION_FILE))?;
        return Ok(Some(ActiveVersion {
          active_type: ActiveType::NodeModules,
          package: PackageDescriptor::parse(paths, &VersionTarget::parse(version)?)?,
          node_modules_path: Some(atlaspack_version_path.clone()),
        }));
      }
    }

    // Detect the system default (unlinked)
    if fs::exists(&paths.global)? {
      let version = fs::read_to_string(paths.global.join(c::APVM_VERSION_FILE))?;
      return Ok(Some(ActiveVersion {
        active_type: ActiveType::SystemDefault,
        package: PackageDescriptor::parse(paths, &VersionTarget::parse(version)?)?,
        node_modules_path: None,
      }));
    }

    Ok(None)
  }
}
