use serde::Serialize;

use crate::public::package_kind::PackageKind;

use super::package::Package;

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum Specifier {
  Npm { version: String },
  Git { version: String },
  Release { version: String },
  Local,
}

impl Serialize for Specifier {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(&format!("{}", self))
  }
}

impl Specifier {
  pub fn parse<S: AsRef<str>>(input: S) -> anyhow::Result<Self> {
    let specifier = input.as_ref();

    if let Some((origin, specifier)) = specifier.split_once(":") {
      return match origin {
        "npm" => Ok(Self::Npm {
          version: specifier.to_string(),
        }),
        "git" => Ok(Self::Git {
          version: specifier.to_string(),
        }),
        "release" => Ok(Self::Release {
          version: specifier.to_string(),
        }),
        "local" => Ok(Self::Local),
        _ => Err(anyhow::anyhow!("Cannot parse version specifier")),
      };
    };

    if specifier == "local" {
      return Ok(Self::Local);
    }

    Ok(Self::Npm {
      version: specifier.to_string(),
    })
  }

  pub fn version(&self) -> &str {
    match self {
      Specifier::Npm { version } => version,
      Specifier::Git { version } => version,
      Specifier::Release { version } => version,
      Specifier::Local => "",
    }
  }

  pub fn kind(&self) -> PackageKind {
    match self {
      Specifier::Npm { version: _ } => PackageKind::Npm,
      Specifier::Git { version: _ } => PackageKind::Git,
      Specifier::Release { version: _ } => PackageKind::Release,
      Specifier::Local => PackageKind::Local,
    }
  }
}

impl From<Specifier> for String {
  fn from(value: Specifier) -> Self {
    format!("{}", value)
  }
}

impl TryFrom<&Package> for Specifier {
  type Error = anyhow::Error;

  fn try_from(value: &Package) -> Result<Self, Self::Error> {
    match value {
      Package::Local(_) => Ok(Self::Local),
      Package::Npm(package) => Ok(Self::Npm {
        version: package.version.clone(),
      }),
      Package::Git(package) => Ok(Self::Git {
        version: package.version.clone(),
      }),
      Package::Release(package) => Ok(Self::Release {
        version: package.version.clone(),
      }),
      Package::Unmanaged(_package) => Err(anyhow::anyhow!(
        "Cannot parse specifier of unmanaged version"
      )),
      Package::Default(package) => match &package.meta.specifier {
        Some(specifier) => Ok(Self::parse(specifier)?),
        None => Err(anyhow::anyhow!("Meta has no specifier")),
      },
    }
  }
}

impl std::fmt::Display for Specifier {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Specifier::Npm { version } => write!(f, "{}", version),
      Specifier::Git { version: branch } => write!(f, "git:{}", branch),
      Specifier::Release { version: branch } => write!(f, "release:{}", branch),
      Specifier::Local => write!(f, "local"),
    }
  }
}
