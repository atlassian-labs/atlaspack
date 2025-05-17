use std::collections::HashMap;

use crate::{
  env::Env,
  platform::{apvmrc::ApvmRc, package::Package, path_ext::PathExt, specifier::Specifier},
  public::{apvm_config::ApvmConfig, json_serde::JsonSerde},
};

#[derive(Debug, Clone)]
pub struct Validator {
  apvmrc: Option<ApvmRc>,
  env: Env,
}

impl Validator {
  pub fn new(env: &Env, apvmrc: &Option<ApvmRc>) -> Self {
    Self {
      apvmrc: apvmrc.clone(),
      env: env.clone(),
    }
  }

  pub fn checksum_verify(&self, package: &Package, checksum: &str) -> bool {
    match package {
      Package::Local(_local_package) => true,
      Package::Npm(npm_package) => match &npm_package.checksum {
        Some(package_checksum) => package_checksum == checksum,
        None => true,
      },
      Package::Git(_git_package) => true,
      Package::Release(release_package) => match &release_package.checksum {
        Some(package_checksum) => package_checksum == checksum,
        None => true,
      },
      Package::Unmanaged(_unmanaged_package) => true,
      Package::Default(_default_package) => true,
    }
  }

  pub fn verify_package_checksum(&self, package: &Package) -> anyhow::Result<bool> {
    let Some(apvmrc) = &self.apvmrc else {
      return Ok(true);
    };

    let specifier = Specifier::try_from(package)?;
    let Some(checksum) = apvmrc.checksums.get(&specifier) else {
      return Ok(true);
    };

    Ok(self.checksum_verify(package, checksum))
  }

  pub fn update_checksum(
    &self,
    specifier: &Specifier,
    checksum: impl AsRef<str>,
  ) -> anyhow::Result<()> {
    let checksum = checksum.as_ref();

    let apvmrc = match &self.apvmrc {
      Some(apvmrc) => ApvmRc::detect(apvmrc.path.try_parent()?)?.unwrap(),
      None => ApvmRc {
        path: self.env.pwd.join("apvm.json"),
        versions: Default::default(),
        checksums: Default::default(),
      },
    };

    let mut versions = HashMap::<String, String>::new();
    let mut checksums = HashMap::<String, String>::new();

    for (version, specifier) in &apvmrc.versions {
      versions.insert(version.clone(), specifier.to_string());
    }

    'block: for (specifier, checksum) in &apvmrc.checksums {
      for (_, included) in &apvmrc.versions {
        if included == specifier {
          checksums.insert(specifier.to_string(), checksum.clone());
          continue 'block;
        }
      }
    }
    checksums.insert(specifier.to_string(), checksum.to_string());

    ApvmConfig::write_to_file(
      &ApvmConfig {
        version: None,
        versions: Some(versions),
        checksums: Some(checksums),
      },
      &apvmrc.path,
    )?;

    Ok(())
  }

  pub fn set_alias(&self, name: &str, version: &str) -> anyhow::Result<()> {
    let apvmrc = match &self.apvmrc {
      Some(apvmrc) => ApvmRc::detect(apvmrc.path.try_parent()?)?.unwrap(),
      None => ApvmRc {
        path: self.env.pwd.join("apvm.json"),
        versions: Default::default(),
        checksums: Default::default(),
      },
    };

    let mut versions = HashMap::<String, String>::new();
    let mut checksums = HashMap::<String, String>::new();

    for (version, specifier) in &apvmrc.versions {
      versions.insert(version.clone(), specifier.to_string());
    }

    versions.insert(name.to_string(), version.to_string());

    for (specifier, checksum) in &apvmrc.checksums {
      checksums.insert(specifier.to_string(), checksum.clone());
    }

    ApvmConfig::write_to_file(
      &ApvmConfig {
        version: None,
        versions: Some(versions),
        checksums: Some(checksums),
      },
      &apvmrc.path,
    )?;

    Ok(())
  }
}
