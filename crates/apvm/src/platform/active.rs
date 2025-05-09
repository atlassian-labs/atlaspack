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
    log::info!(
      "Looking for node_modules {:?}",
      paths.node_modules_atlaspack
    );
    if let Some(node_modules) = &paths.node_modules_apvm {
      let breadcrumb = node_modules.join(c::APVM_VERSION_FILE);
      let version = fs::read_to_string(&breadcrumb)?;

      if fs::exists(&breadcrumb)? {
        let version_target = VersionTarget::parse(version, paths)?;

        let alias: Option<String> = 'block: {
          if let Some(apvmrc) = apvmrc {
            for (alias, version) in &apvmrc.version_aliases {
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
          node_modules_path: Some(node_modules.clone()),
          alias,
        }));
      }
    }

    // Detect the system default (unlinked)
    log::info!("Checking if system version is set {:?}", paths.global);
    let version_file = paths.global.join(c::APVM_VERSION_FILE);
    if fs::exists(&paths.global)? && fs::exists(&version_file)? {
      let version = fs::read_to_string(&version_file)?;
      return Ok(Some(ActiveVersion {
        active_type: ActiveType::SystemDefault,
        package: PackageDescriptor::parse(paths, &VersionTarget::parse(version, paths)?)?,
        node_modules_path: None,
        alias: None,
      }));
    }

    Ok(None)
  }
}
