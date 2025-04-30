use serde::Serialize;

use crate::platform::apvmrc::ApvmRc;
use crate::platform::package::InstallablePackage;
use crate::platform::package::ManagedPackage;
use crate::platform::package::Package;
use crate::platform::specifier::Specifier;
use crate::versions::Versions;

#[derive(Clone, Serialize)]
pub struct PackageResolver {
  apvmrc: Option<ApvmRc>,
  versions: Versions,
}

impl std::fmt::Debug for PackageResolver {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "PackageResolver{{}}")
  }
}

impl PackageResolver {
  pub fn new(apvmrc: &Option<ApvmRc>, versions: &Versions) -> Self {
    Self {
      apvmrc: apvmrc.clone(),
      versions: versions.clone(),
    }
  }
  /// Convert a parse text into an specifier, resolving to an underlying alias in the apvmrc
  pub fn resolve_specifier<S: AsRef<str>>(&self, input: S) -> anyhow::Result<Specifier> {
    let input = input.as_ref();

    if input == "local" {
      return Ok(Specifier::Local);
    } else if let Some(apvmrc) = &self.apvmrc {
      if let Some(alias) = apvmrc.versions.get(input) {
        return Ok(alias.clone());
      }
    }

    Specifier::parse(input)
  }

  /// Resolve a specifier to an installed version
  pub fn resolve(&self, input: &Specifier) -> Option<ManagedPackage> {
    if *input == Specifier::Local {
      if let Some(local) = &self.versions.local {
        return Some(ManagedPackage::Local(local.clone()));
      }
    } else {
      for installed in &self.versions.installed {
        match &installed {
          InstallablePackage::Npm(package) => {
            if let Specifier::Npm { version } = &input {
              if &package.version == version {
                return Some(ManagedPackage::Npm(package.clone()));
              };
            };
          }
          InstallablePackage::Release(package) => {
            if let Specifier::Release { version } = &input {
              if &package.version == version {
                return Some(ManagedPackage::Release(package.clone()));
              };
            };
          }
          InstallablePackage::Git(package) => {
            if let Specifier::Git { version: branch } = &input {
              if &package.version == branch {
                return Some(ManagedPackage::Git(package.clone()));
              };
            };
          }
        }
      }
    }

    None
  }

  /// Resolve a specifier to an installed package
  ///
  /// Order:
  /// - Specified version
  /// - node_modules
  /// - Apvmrc default
  /// - system default
  pub fn resolve_or_default(&self, input: &Option<String>) -> anyhow::Result<Package> {
    let mut specifier: Option<Specifier> = None;

    log::info!("resolving: {:?}", input);

    if let Some(version) = input {
      specifier = Some(self.resolve_specifier(version)?)
    } else if let Some(result) = &self.versions.node_modules {
      return Ok(result.clone());
    } else if let Some(apvmrc) = &self.apvmrc {
      if let Some(default) = apvmrc.versions.get("default") {
        specifier = Some(default.clone())
      }
    }

    log::info!("resolved: {:?}", specifier);

    if let Some(specifier) = &specifier {
      if let Some(result) = self.resolve(specifier) {
        return Ok(match result {
          ManagedPackage::Npm(package) => Package::Npm(package),
          ManagedPackage::Release(package) => Package::Release(package),
          ManagedPackage::Git(package) => Package::Git(package),
          ManagedPackage::Local(package) => Package::Local(package),
        });
      } else {
        return Err(anyhow::anyhow!("Version not installed {}", specifier));
      }
    } else if let Some(result) = &self.versions.default {
      return Ok(Package::Default(result.clone()));
    }

    Err(anyhow::anyhow!("Version not found"))
  }
}
