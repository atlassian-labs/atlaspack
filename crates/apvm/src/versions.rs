use std::fs;

use serde::Deserialize;
use serde::Serialize;

use crate::paths::Paths;
use crate::platform::apvmrc::ApvmRc;
use crate::platform::constants as c;
use crate::platform::package::DefaultPackage;
use crate::platform::package::GitPackage;
use crate::platform::package::InstallablePackage;
use crate::platform::package::LocalPackage;
use crate::platform::package::NpmPackage;
use crate::platform::package::Package;
use crate::platform::package::ReleasePackage;
use crate::platform::package::UnmanagedPackage;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

/// A virtual representation of the packages available locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versions {
  /// The version currently selected by apvm
  ///
  /// Priority:
  /// - node_modules
  /// - global default
  pub active: Option<Package>,
  /// The version of atlaspack in node_modules.
  /// This could be linked or "unmanaged" if
  /// installed without apvm
  pub node_modules: Option<Package>,
  /// This is the version of Atlaspack set
  /// globally as the default version to
  /// fallback on
  pub default: Option<DefaultPackage>,
  /// This is a local copy of Atlaspack sources
  pub local: Option<LocalPackage>,
  /// This is a list of Atlaspack versions installed
  /// and available on the system
  pub installed: Vec<InstallablePackage>,
}

impl Versions {
  pub fn detect(paths: &Paths, apvmrc: &Option<ApvmRc>) -> anyhow::Result<Self> {
    let installed = detect_installed_versions(paths, apvmrc)?;
    let node_modules = detect_node_modules_version(paths, apvmrc)?;
    let local = detect_local_version(paths);
    let default = detect_default_version(paths)?;

    let active = match &node_modules {
      Some(package) => Some(package.clone()),
      None => match &default {
        Some(package) => Some(Package::Default(package.clone())),
        None => None,
      },
    };

    Ok(Versions {
      active,
      node_modules,
      default,
      local,
      installed,
    })
  }
}

/// Detect if Atlaspack is installed in node_modules
/// and if it is determine if it's managed by apvm
fn detect_node_modules_version(
  paths: &Paths,
  apvmrc: &Option<ApvmRc>,
) -> anyhow::Result<Option<Package>> {
  let Some(node_modules) = &paths.node_modules else {
    return Ok(None);
  };

  // Check for an "node_modules/.apvm/atlaspack_version" file
  if let Some(node_modules_apvm) = &paths.node_modules_apvm {
    let meta_file = node_modules_apvm.join(c::LINK_META_FILE);
    if !fs::exists(&meta_file)? {
      return Ok(None);
    }

    let meta = LinkedMeta::read_from_file(&meta_file)?;
    let version = meta.version.clone().unwrap_or_default();
    let alias = lookup_package_has_alias(&version, apvmrc);

    let specifier = if let Some(specifier) = &meta.specifier {
      Specifier::parse(specifier)?
    } else {
      Specifier::parse("")?
    };

    return Ok(Some(match meta.kind {
      PackageKind::Npm => Package::Npm(NpmPackage {
        specifier: specifier.to_string(),
        version: specifier.version().to_string(),
        path: meta.original_path,
        alias,
        checksum: None,
      }),
      PackageKind::Release => Package::Release(ReleasePackage {
        specifier: specifier.to_string(),
        version: specifier.version().to_string(),
        path: meta.original_path,
        alias,
        checksum: None,
      }),
      PackageKind::Git => Package::Git(GitPackage {
        specifier: specifier.to_string(),
        version: specifier.version().to_string(),
        path: meta.original_path,
        alias,
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
  paths
    .atlaspack_local
    .as_ref()
    .map(|atlaspack_local| LocalPackage {
      path: atlaspack_local.clone(),
    })
}

/// Read the install directory and load the installed versions
fn detect_installed_versions(
  paths: &Paths,
  apvmrc: &Option<ApvmRc>,
) -> anyhow::Result<Vec<InstallablePackage>> {
  let mut versions_installed = Vec::<InstallablePackage>::new();

  for entry in std::fs::read_dir(&paths.versions_v1)? {
    let path = entry?.path();
    let meta = path.join("meta.json");

    let package_meta = PackageMeta::read_from_file(&meta)?;
    let version = package_meta.version.clone().unwrap_or_default();
    let alias = lookup_package_has_alias(&version, apvmrc);
    let specifier = if let Some(specifier) = &package_meta.specifier {
      Specifier::parse(specifier)?
    } else {
      Specifier::parse("")?
    };

    versions_installed.push(match package_meta.kind {
      PackageKind::Npm => InstallablePackage::Npm(NpmPackage {
        specifier: specifier.to_string(),
        version: specifier.version().to_string(),
        path,
        alias,
        checksum: package_meta.checksum,
      }),
      PackageKind::Release => InstallablePackage::Release(ReleasePackage {
        specifier: specifier.to_string(),
        version: specifier.version().to_string(),
        path,
        alias,
        checksum: package_meta.checksum,
      }),
      PackageKind::Git => InstallablePackage::Git(GitPackage {
        specifier: specifier.to_string(),
        version: specifier.version().to_string(),
        path,
        alias,
      }),
      PackageKind::Local => unreachable!(),
      PackageKind::Unmanaged => unreachable!(),
    });
  }

  Ok(versions_installed)
}

/// Detect if there is a version installed globally to use as a fallback
fn detect_default_version(paths: &Paths) -> anyhow::Result<Option<DefaultPackage>> {
  if !fs::exists(&paths.global)? {
    return Ok(None);
  }

  let Ok(package) = DefaultPackage::new_from_dir(&paths.global) else {
    fs::remove_dir_all(&paths.global)?;
    return Ok(None);
  };

  Ok(Some(package))
}

/// Lookup if the version of a package has an alias defined in the current config
fn lookup_package_has_alias(incoming_versions: &String, apvmrc: &Option<ApvmRc>) -> Option<String> {
  let Some(apvmrc) = apvmrc else {
    return None;
  };

  for (incoming_alias, specifier) in &apvmrc.versions {
    let version = match specifier {
      Specifier::Npm { version } => version,
      Specifier::Git { version } => version,
      Specifier::Release { version } => version,
      Specifier::Local => continue,
    };

    if version == incoming_versions {
      return Some(incoming_alias.clone());
    }
  }

  None
}
