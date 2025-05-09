use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use super::apvmrc::ApvmRc;
use super::constants as c;
use super::origin::VersionTarget;
use super::package::PackageDescriptor;
use crate::paths::Paths;

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub enum ActiveType {
  SystemDefault,
  NodeModules,
}

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub struct ActiveVersion {
  pub active_type: ActiveType,
  pub alias: Option<String>,
  pub package: PackageDescriptor,
  pub node_modules_path: Option<PathBuf>,
}

impl ActiveVersion {
  pub fn detect(apvmrc: &Option<ApvmRc>, paths: &Paths) -> anyhow::Result<Option<Self>> {
    // Detect from node_modules
    log::info!("Testing Local {:?}", paths.node_modules_atlaspack);
    if let Some(atlaspack_version_path) = &paths.node_modules_atlaspack {
      let version_path = atlaspack_version_path.join(c::APVM_VERSION_FILE);
      log::info!("Version Path {:?}", version_path);

      if fs::exists(&version_path)? {
        let version = fs::read_to_string(atlaspack_version_path.join(c::APVM_VERSION_FILE))?;
        let version_target = VersionTarget::parse(version)?;

        let alias: Option<String> = 'block: {
          if let Some(apvmrc) = apvmrc {
            for (alias, version) in &apvmrc.version_target_aliases {
              if version == &version_target {
                break 'block Some(alias.clone());
              }
            }
          }
          None
        };

        return Ok(Some(ActiveVersion {
          active_type: ActiveType::NodeModules,
          package: PackageDescriptor::parse(paths, &version_target)?,
          node_modules_path: Some(atlaspack_version_path.clone()),
          alias,
        }));
      }
    }

    // Detect the system default (unlinked)
    log::info!("Testing Global {:?}", paths.global);
    let version_file = paths.global.join(c::APVM_VERSION_FILE);
    if fs::exists(&paths.global)? && fs::exists(&version_file)? {
      let version = fs::read_to_string(&version_file)?;
      return Ok(Some(ActiveVersion {
        active_type: ActiveType::SystemDefault,
        package: PackageDescriptor::parse(paths, &VersionTarget::parse(version)?)?,
        node_modules_path: None,
        alias: None,
      }));
    }

    Ok(None)
  }
}
