use std::fs;

use serde::{Deserialize, Serialize};

use crate::paths::Paths;
use crate::platform::constants as c;
use crate::platform::package::GitPackage;
use crate::platform::package::InstallablePackage;
use crate::platform::package::LocalPackage;
use crate::platform::package::NpmPackage;
use crate::platform::package::Package;
use crate::platform::package::UnmanagedPackage;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

/// A virtual representation of the packages available locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versions {
  pub node_modules: Option<Package>,
  pub local: Option<LocalPackage>,
  pub installed: Vec<InstallablePackage>,
}

impl Versions {
  pub fn detect(paths: &Paths) -> anyhow::Result<Self> {
    let node_modules = detect_node_modules_version(paths)?;
    let local = detect_local_version(paths);
    let installed = detect_installed_versions(paths)?;

    Ok(Versions {
      node_modules,
      local,
      installed,
    })
  }
}

/// Detect if Atlaspack is installed in node_modules
/// and if it is determine if it's managed by apvm
fn detect_node_modules_version(paths: &Paths) -> anyhow::Result<Option<Package>> {
  let Some(node_modules) = &paths.node_modules else {
    return Ok(None);
  };

  // Check for an "node_modules/.apvm/atlaspack_version" file
  if let Some(node_modules_apvm) = &paths.node_modules_apvm {
    let meta_file = node_modules_apvm.join(c::APVM_VERSION_FILE);
    if !fs::exists(&meta_file)? {
      return Ok(None);
    }

    let meta = LinkedMeta::read_from_file(&meta_file)?;
    return Ok(Some(match meta.kind {
      PackageKind::Npm => Package::Npm(NpmPackage {
        version: meta.specifier.unwrap_or_default(),
        path: meta.original_path,
      }),
      PackageKind::Git => Package::Git(GitPackage {
        branch: meta.specifier.unwrap_or_default(),
        path: meta.original_path,
      }),
      PackageKind::Local => Package::Local(LocalPackage {
        path: meta.original_path,
      }),
      PackageKind::Unmanaged => unreachable!(),
    }));
  }

  // Check for "node_modules/@atlaspack"
  let node_modules_atlaspack_scope = node_modules.join("@atlaspack");
  if fs::exists(&node_modules_atlaspack_scope)? {
    return Ok(Some(Package::Unmanaged(UnmanagedPackage {
      path: node_modules_atlaspack_scope,
    })));
  }

  Ok(None)
}

/// Detect if there is a local version specified in the environment
fn detect_local_version(paths: &Paths) -> Option<LocalPackage> {
  if let Some(atlaspack_local) = &paths.atlaspack_local {
    Some(LocalPackage {
      path: atlaspack_local.clone(),
    })
  } else {
    None
  }
}

/// Read the install directory and load the installed versions
fn detect_installed_versions(paths: &Paths) -> anyhow::Result<Vec<InstallablePackage>> {
  let mut versions_installed = Vec::<InstallablePackage>::new();

  for entry in std::fs::read_dir(&paths.versions_v1)? {
    let path = entry?.path();
    let meta = path.join("meta.json");

    let package_meta = PackageMeta::read_from_file(&meta)?;
    versions_installed.push(match package_meta.kind {
      PackageKind::Npm => InstallablePackage::Npm(NpmPackage {
        version: package_meta.specifier.unwrap_or_default(),
        path,
      }),
      PackageKind::Git => InstallablePackage::Git(GitPackage {
        branch: package_meta.specifier.unwrap_or_default(),
        path,
      }),
      PackageKind::Local => unreachable!(),
      PackageKind::Unmanaged => unreachable!(),
    });
  }

  Ok(versions_installed)
}
