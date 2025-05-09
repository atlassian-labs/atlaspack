use serde::Serialize;

use crate::context::Context;
use crate::paths::Paths;
use crate::platform::path_ext::*;

#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
pub enum VersionTarget {
  Npm(String),
  Git(String),
  Local(String),
  LocalSuper(String),
}

impl std::fmt::Display for VersionTarget {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let s: String = VersionTarget::into(self.clone());
    write!(f, "{s}")
  }
}

impl TryFrom<&str> for VersionTarget {
  type Error = anyhow::Error;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    let Some((origin, specifier)) = value.split_once(":") else {
      if value == "local" {
        return Ok(Self::Local(value.to_string()));
      }
      if value == "local-super" {
        return Ok(Self::Local(value.to_string()));
      }
      if !value.chars().next().unwrap().is_numeric() {
        return Err(anyhow::anyhow!("Invalid semver"));
      }
      return Ok(Self::Npm(value.to_string()));
    };
    match origin {
      "git" => Ok(Self::Git(specifier.to_string())),
      "npm" => Ok(Self::Npm(specifier.to_string())),
      "local" => Ok(Self::Local(specifier.to_string())),
      "local-super" => Ok(Self::Local(specifier.to_string())),
      _ => Err(anyhow::anyhow!("Cannot convert string to VersionTarget")),
    }
  }
}

impl TryFrom<String> for VersionTarget {
  type Error = anyhow::Error;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    Self::try_from(value.as_str())
  }
}

impl From<VersionTarget> for String {
  fn from(val: VersionTarget) -> Self {
    match val {
      VersionTarget::Npm(version) => version.to_string(),
      VersionTarget::Git(version) => format!("git:{}", version),
      VersionTarget::Local(version) => format!("{:?}", version),
      VersionTarget::LocalSuper(version) => format!("{:?}", version),
    }
  }
}

impl VersionTarget {
  pub fn parse<S: AsRef<str>>(value: S, paths: &Paths) -> anyhow::Result<Self> {
    let version = value.as_ref();
    let Some((origin, specifier)) = version.split_once(":") else {
      if version == "local" || version == "local-super" {
        let Some(local_dir) = &paths.atlaspack_local else {
          return Err(anyhow::anyhow!(
            "Requested local but no local version available"
          ));
        };
        if version == "local" {
          return Ok(Self::Local(local_dir.try_to_string()?));
        }
        if version == "local-super" {
          return Ok(Self::LocalSuper(local_dir.try_to_string()?));
        }
      }
      return Ok(Self::Npm(version.to_string()));
    };
    match origin {
      "git" => Ok(Self::Git(specifier.to_string())),
      "npm" => Ok(Self::Npm(specifier.to_string())),
      "local" => Ok(Self::Local(specifier.to_string())),
      "local-super" => Ok(Self::Local(specifier.to_string())),
      _ => Err(anyhow::anyhow!("Cannot convert string to VersionTarget")),
    }
  }

  pub fn resolve(ctx: &Context, version: &Option<String>) -> anyhow::Result<Self> {
    let version = version.clone().unwrap_or_default();
    let apvmrc = ctx.apvmrc.clone().unwrap_or_default();

    if version == "local" || version == "local-super" {
      let Some(local_dir) = &ctx.env.apvm_atlaspack_local else {
        return Err(anyhow::anyhow!(
          "Requested local but no local version available"
        ));
      };
      if version == "local" {
        return Ok(Self::Local(local_dir.try_to_string()?));
      }
      if version == "local-super" {
        return Ok(Self::LocalSuper(local_dir.try_to_string()?));
      }
    };

    if !version.chars().next().unwrap().is_numeric() && !version.starts_with("git:") {
      return Err(anyhow::anyhow!("Invalid version specifier"));
    }

    // Order of target selection:
    // (empty version)  -> package.json#atlaspack.version
    // version          -> package.json#atlaspack.versions[version]
    // version          -> lookup (git|npm|local)[version]

    // If the version is empty and there is a default specified in the apvmrc
    if version.is_empty() {
      if let Some(target) = apvmrc.version_aliases.get("default") {
        return Ok(target.clone());
      }
      return Err(anyhow::anyhow!("No version selected"));
    }

    // If the version is specified and it matches an alias in the apvmrc
    if let Some(target) = apvmrc.version_aliases.get(&version) {
      return Ok(target.clone());
    }

    // Resolve the version specified
    Self::try_from(version.as_str())
  }

  pub fn version(&self) -> &str {
    match self {
      VersionTarget::Npm(version) => version,
      VersionTarget::Git(version) => version,
      VersionTarget::Local(version) => version,
      VersionTarget::LocalSuper(version) => version,
    }
  }

  pub fn origin(&self) -> &str {
    match self {
      VersionTarget::Npm(_version) => "npm",
      VersionTarget::Git(_version) => "git",
      VersionTarget::Local(_version) => "local",
      VersionTarget::LocalSuper(_version) => "local-super",
    }
  }
}
