use std::fs;

use serde::Serialize;

use crate::paths::Paths;
use crate::platform::apvmrc::ApvmRcRef;
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
#[derive(Debug, Clone)]
pub struct Versions {
  paths: Paths,
  apvmrc: ApvmRcRef,
}

impl Versions {
  pub fn new(paths: &Paths, apvmrc: &ApvmRcRef) -> anyhow::Result<Self> {
    Ok(Versions {
      paths: paths.clone(),
      apvmrc: apvmrc.clone(),
    })
  }

  /// List of Atlaspack versions installed and available on the system
  pub fn installed(&self) -> anyhow::Result<Vec<InstallablePackage>> {
    let mut versions_installed = Vec::<InstallablePackage>::new();

    for entry in std::fs::read_dir(&self.paths.versions_v1).map_err(|e| {
      anyhow::anyhow!(
        "Failed to read versions directory {:?} in versions.rs:installed(): {}",
        self.paths.versions_v1,
        e
      )
    })? {
      let path = entry?.path();
      let meta = path.join("meta.json");

      let package_meta = PackageMeta::read_from_file(&meta)?;
      let version = package_meta.version.clone().unwrap_or_default();
      let alias = self.lookup_package_has_alias(&version);
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

  /// This is a local copy of Atlaspack sources
  pub fn local(&self) -> Option<LocalPackage> {
    self
      .paths
      .atlaspack_local
      .as_ref()
      .map(|atlaspack_local| LocalPackage {
        path: atlaspack_local.clone(),
      })
  }

  /// Detect if Atlaspack is installed in node_modules
  /// and if it is determine if it's managed by apvm
  pub fn node_modules(&self) -> anyhow::Result<Option<Package>> {
    let Some(node_modules) = &self.paths.node_modules else {
      return Ok(None);
    };

    // Check for an "node_modules/.apvm/atlaspack_version" file
    if let Some(node_modules_apvm) = &self.paths.node_modules_apvm {
      let meta_file = node_modules_apvm.join(c::LINK_META_FILE);
      if !fs::exists(&meta_file)? {
        return Ok(None);
      }

      let meta = LinkedMeta::read_from_file(&meta_file)?;
      let version = meta.version.clone().unwrap_or_default();
      let alias = self.lookup_package_has_alias(&version);

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

  /// The version currently selected by apvm
  /// Priority:
  /// - node_modules
  /// - global default
  pub fn active(&self) -> anyhow::Result<Option<Package>> {
    match self.node_modules()? {
      Some(package) => Ok(Some(package)),
      None => match self.default()? {
        Some(package) => Ok(Some(Package::Default(package))),
        None => Ok(None),
      },
    }
  }

  /// This is the version of Atlaspack set globally as the default version to
  /// fallback on
  pub fn default(&self) -> anyhow::Result<Option<DefaultPackage>> {
    if !fs::exists(&self.paths.global)? {
      return Ok(None);
    }

    let Ok(package) = DefaultPackage::new_from_dir(&self.paths.global) else {
      fs::remove_dir_all(&self.paths.global)?;
      return Ok(None);
    };

    Ok(Some(package))
  }

  /// Lookup if the version of a package has an alias defined in the current config
  fn lookup_package_has_alias(&self, incoming_versions: &String) -> Option<String> {
    if !self.apvmrc.exists {
      return None;
    };

    for (incoming_alias, specifier) in self.apvmrc.get_aliases() {
      let version = match specifier {
        Specifier::Npm { version } => version,
        Specifier::Git { version } => version,
        Specifier::Release { version } => version,
        Specifier::Local => continue,
      };

      if version == *incoming_versions {
        return Some(incoming_alias.clone());
      }
    }

    None
  }
}

impl Serialize for Versions {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct VersionsSerde {
      pub active: Option<Package>,
      pub node_modules: Option<Package>,
      pub default: Option<DefaultPackage>,
      pub local: Option<LocalPackage>,
      pub installed: Vec<InstallablePackage>,
    }
    VersionsSerde {
      active: self.active().unwrap(),
      node_modules: self.node_modules().unwrap(),
      default: self.default().unwrap(),
      local: self.local(),
      installed: self.installed().unwrap(),
    }
    .serialize(serializer)
  }
}
