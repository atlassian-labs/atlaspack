use crate::platform::apvmrc::ApvmRcRef;
use crate::platform::package::Package;
use crate::platform::specifier::Specifier;

#[derive(Debug, Clone)]
pub struct Validator {
  apvmrc: ApvmRcRef,
}

impl Validator {
  pub fn new(apvmrc: &ApvmRcRef) -> Self {
    Self {
      apvmrc: apvmrc.clone(),
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
    if !self.apvmrc.exists {
      return Ok(true);
    }

    let specifier = Specifier::try_from(package)?;
    let Some(checksum) = self.apvmrc.get_checksum(&specifier) else {
      return Ok(true);
    };

    Ok(self.checksum_verify(package, &checksum))
  }
}
