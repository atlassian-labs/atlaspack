use std::fs;

use serde::{Deserialize, Serialize};

use crate::paths::Paths;
use crate::platform::apvmrc::ApvmRc;
use crate::platform::constants as c;
use crate::platform::linked_meta::LinkedMeta;
use crate::platform::package_kind::PackageKind;
use crate::platform::package_meta::PackageMeta;
use crate::platform::package_path::PackagePath;
use crate::platform::package_path::PackagePathUnmanaged;
use crate::platform::specifier::parse_specifier;
use crate::platform::version::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versions {
  #[serde(skip)]
  apvmrc: Option<ApvmRc>,
  pub default: Option<DefaultVersion>,
  pub node_modules: Option<Version>,
  pub local: Option<Version>,
  pub installed: Vec<Version>,
}

impl Versions {
  pub fn detect(paths: &Paths, apvmrc: &Option<ApvmRc>) -> anyhow::Result<Self> {
    let node_modules: Option<Version> = detect_node_modules_version(paths)?;
    let local = detect_local_version(paths);
    let installed = detect_installed_versions(paths)?;
    let default = detect_default_version(&node_modules);

    Ok(Versions {
      apvmrc: apvmrc.clone(),
      default,
      node_modules,
      local,
      installed,
    })
  }

  pub fn resolve<S: AsRef<str>>(
    &self,
    specifier: S,
  ) -> anyhow::Result<Option<VersionResolveResult>> {
    if let Some(result) = self.resolve_alias(&specifier)? {
      return Ok(Some(result));
    }

    if let Some(result) = self.resolve_installed(&specifier)? {
      return Ok(Some(VersionResolveResult::Installed(result)));
    }

    Ok(None)
  }

  pub fn resolve_alias<S: AsRef<str>>(
    &self,
    specifier: S,
  ) -> anyhow::Result<Option<VersionResolveResult>> {
    let Some(apvmrc) = &self.apvmrc else {
      return Ok(None);
    };

    let specifier = specifier.as_ref();

    for (alias, package_meta) in &apvmrc.version_aliases {
      if specifier == alias {
        let Some(package_specifier) = &package_meta.specifier else {
          return Err(anyhow::anyhow!(
            "Cannot set an alias to a package type without specifier"
          ));
        };

        return match self.resolve_installed(package_specifier)? {
          Some(version) => Ok(Some(VersionResolveResult::Installed(version))),
          None => Ok(Some(VersionResolveResult::NotInstalled {
            specifier: package_specifier.clone(),
            kind: package_meta.kind.clone(),
          })),
        };
      }
    }

    Ok(None)
  }

  pub fn resolve_installed<S: AsRef<str>>(&self, specifier: S) -> anyhow::Result<Option<Version>> {
    let (kind, specifier) = parse_specifier(specifier)?;

    for version in &self.installed {
      if version.kind == kind && version.specifier == specifier {
        return Ok(Some(version.clone()));
      }
    }

    Ok(None)
  }
}

/// Detect if Atlaspack is installed in node_modules
/// and if it is determine if it's managed by apvm
fn detect_node_modules_version(paths: &Paths) -> anyhow::Result<Option<Version>> {
  let Some(node_modules) = &paths.node_modules else {
    return Ok(None);
  };

  let Some(node_modules_apvm) = &paths.node_modules_apvm else {
    let node_modules_atlaspack_scope = node_modules.join("@atlaspack");
    if fs::exists(&node_modules_atlaspack_scope)? {
      return Ok(Some(Version {
        kind: PackageKind::Unmanaged,
        specifier: None,
        path: PackagePath::Unmanaged(PackagePathUnmanaged {
          base: node_modules_atlaspack_scope.clone(),
        }),
      }));
    }
    return Ok(None);
  };

  let meta_file = node_modules_apvm.join(c::APVM_VERSION_FILE);
  if !fs::exists(&meta_file)? {
    return Ok(None);
  }

  let meta = LinkedMeta::read_from_file(&meta_file)?;
  Ok(Some(Version {
    kind: meta.kind,
    specifier: meta.specifier,
    path: meta.original_path,
  }))
}

/// Detect if there is a local version specified in the environment
fn detect_local_version(paths: &Paths) -> Option<Version> {
  if let Some(atlaspack_local) = &paths.atlaspack_local {
    Some(Version {
      kind: PackageKind::Local,
      specifier: None,
      path: PackagePath::local_from_path(atlaspack_local),
    })
  } else {
    None
  }
}

/// Read the install directory and load the installed versions
fn detect_installed_versions(paths: &Paths) -> anyhow::Result<Vec<Version>> {
  let mut versions_installed = Vec::<Version>::new();

  for entry in std::fs::read_dir(&paths.versions_v1)? {
    let entry = entry?.path();
    let package_path = PackagePath::installed_from_path(&entry);

    let PackagePath::Installed(pkg) = &package_path else {
      unreachable!()
    };

    let package_meta = PackageMeta::read_from_file(&pkg.meta)?;
    versions_installed.push(Version {
      kind: package_meta.kind,
      path: package_path,
      specifier: package_meta.specifier,
    })
  }

  Ok(versions_installed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionResolveResult {
  Installed(Version),
  NotInstalled {
    specifier: String,
    kind: PackageKind,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultVersionLocation {
  NodeModules,
  Global,
  Unlinked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultVersion {
  pub location: DefaultVersionLocation,
  pub version: Version,
}

/// Version to fallback to if none are selection
pub fn detect_default_version(version_node_modules: &Option<Version>) -> Option<DefaultVersion> {
  if let Some(version_node_modules) = version_node_modules {
    return Some(DefaultVersion {
      location: DefaultVersionLocation::NodeModules,
      version: version_node_modules.clone(),
    });
  }
  None
}
