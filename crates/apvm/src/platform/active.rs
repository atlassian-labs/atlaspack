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
  Unmanaged,
}

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub struct ActiveVersion {
  pub active_type: ActiveType,
  pub alias: Option<String>,
  pub package: Option<PackageDescriptor>,
  pub bin_path: PathBuf,
}

impl ActiveVersion {
  pub fn detect(apvmrc: &Option<ApvmRc>, paths: &Paths) -> anyhow::Result<Option<Self>> {
    // Detect from node_modules
    log::info!(
      "Looking for node_modules {:?}",
      paths.node_modules_atlaspack
    );
    if let Some(node_modules) = &paths.node_modules {
      log::info!("Found {:?}", node_modules);
      if let Some(node_modules_apvm) = &paths.node_modules_apvm {
        log::info!("Found {:?}", node_modules_apvm);
        let breadcrumb = node_modules_apvm.join(c::APVM_VERSION_FILE);
        if fs::exists(&breadcrumb)? {
          log::info!("Found {:?}", breadcrumb);
          let version = fs::read_to_string(&breadcrumb)?;
          let version_target = VersionTarget::parse(version, paths)?;
          let package = PackageDescriptor::parse(paths, &version_target)?;
          let bin_path = package.path.join(&package.bin_path);

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
            package: Some(package),
            alias,
            bin_path,
          }));
        }
      } else {
        let atlaspack_legacy = node_modules.join("@atlaspack").join("core");
        if fs::exists(&atlaspack_legacy)? {
          log::info!("Found {:?}", atlaspack_legacy);

          return Ok(Some(ActiveVersion {
            active_type: ActiveType::Unmanaged,
            package: None,
            alias: None,
            bin_path: node_modules.join(".bin").join("atlaspack"),
          }));
        }
      }
    }

    // Detect the system default (unlinked)
    log::info!("Looking for default version {:?}", paths.global);
    if fs::exists(&paths.global)? && fs::exists(&paths.global_version)? {
      log::info!("Found {:?}", paths.global);
      let version = fs::read_to_string(&paths.global_version)?;
      let version_target = VersionTarget::parse(version, paths)?;
      let package = PackageDescriptor::parse(paths, &version_target)?;
      let bin_path = package.path.join(&package.bin_path);

      return Ok(Some(ActiveVersion {
        active_type: ActiveType::SystemDefault,
        package: Some(package),
        alias: None,
        bin_path,
      }));
    }

    Ok(None)
  }
}
