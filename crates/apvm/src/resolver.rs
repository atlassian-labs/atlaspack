use serde::Serialize;

use crate::platform::apvmrc::ApvmRcRef;
use crate::platform::package::InstallablePackage;
use crate::platform::package::ManagedPackage;
use crate::platform::package::Package;
use crate::platform::specifier::Specifier;
use crate::versions::Versions;

#[derive(Clone, Serialize)]
pub struct PackageResolver {
  apvmrc: ApvmRcRef,
  versions: Versions,
}

impl std::fmt::Debug for PackageResolver {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "PackageResolver{{}}")
  }
}

impl PackageResolver {
  pub fn new(apvmrc: &ApvmRcRef, versions: &Versions) -> Self {
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
    } else if self.apvmrc.exists
      && let Some(alias) = self.apvmrc.get_alias(input)
    {
      return Ok(alias.clone());
    }

    Specifier::parse(input)
  }

  /// Resolve a specifier to an installed version
  pub fn resolve(&self, input: &Specifier) -> anyhow::Result<Option<ManagedPackage>> {
    if *input == Specifier::Local {
      if let Some(local) = self.versions.local() {
        return Ok(Some(ManagedPackage::Local(local)));
      }
    } else {
      for installed in self.versions.installed()? {
        match installed {
          InstallablePackage::Npm(package) => {
            if let Specifier::Npm { version } = &input
              && &package.version == version
            {
              return Ok(Some(ManagedPackage::Npm(package)));
            };
          }
          InstallablePackage::Release(package) => {
            if let Specifier::Release { version } = &input
              && &package.version == version
            {
              return Ok(Some(ManagedPackage::Release(package)));
            };
          }
          InstallablePackage::Git(package) => {
            if let Specifier::Git { version: branch } = &input
              && &package.version == branch
            {
              return Ok(Some(ManagedPackage::Git(package)));
            };
          }
        }
      }
    }

    Ok(None)
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
    } else if let Some(result) = self.versions.node_modules()? {
      return Ok(result);
    } else if self.apvmrc.exists
      && let Some(default) = self.apvmrc.get_alias("default")
    {
      specifier = Some(default)
    }

    log::info!("resolved: {:?}", specifier);

    if let Some(specifier) = &specifier {
      if let Some(result) = self.resolve(specifier)? {
        return Ok(match result {
          ManagedPackage::Npm(package) => Package::Npm(package),
          ManagedPackage::Release(package) => Package::Release(package),
          ManagedPackage::Git(package) => Package::Git(package),
          ManagedPackage::Local(package) => Package::Local(package),
        });
      } else {
        return Err(anyhow::anyhow!("Version not installed {}", specifier));
      }
    } else if let Some(result) = self.versions.default()? {
      return Ok(Package::Default(result));
    }

    Err(anyhow::anyhow!("Version not found"))
  }
}
