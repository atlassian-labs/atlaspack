use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use super::name;
use super::origin::VersionTarget;
use super::path_ext::*;
use crate::paths::Paths;

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackageDescriptor {
  pub version_target: VersionTarget,
  pub origin: String,
  pub version: String,
  pub version_encoded: String,
  pub path: PathBuf,
  pub bin_path: PathBuf,
}

impl PackageDescriptor {
  /// Create a PackageDescriptor from a version target.
  /// This file might not exist
  pub fn parse(paths: &Paths, version_target: &VersionTarget) -> anyhow::Result<Self> {
    if let VersionTarget::Local(_path) = version_target {
      if let Some(atlaspack_local) = &paths.atlaspack_local {
        return Ok(Self {
          version_target: version_target.clone(),
          origin: version_target.origin().to_string(),
          version: version_target.version().to_string(),
          version_encoded: "".to_string(),
          path: atlaspack_local.clone(),
          bin_path: PathBuf::from_iter(&["packages", "core", "cli", "lib", "cli.js"]),
        });
      } else {
        return Err(anyhow::anyhow!("No local version registered"));
      }
    };

    let version = version_target.version();
    let version_encoded = name::encode(version)?;
    let path = paths
      .versions
      .join(version_target.origin())
      .join(&version_encoded);

    let bin_path = match version_target {
      VersionTarget::Npm(_) => PathBuf::from_iter(&["lib", "cli.js"]),
      _ => PathBuf::from_iter(&["packages", "core", "cli", "lib", "cli.js"]),
    };

    Ok(Self {
      version_target: version_target.clone(),
      origin: version_target.origin().to_string(),
      version: version.to_string(),
      version_encoded,
      path,
      bin_path,
    })
  }

  /// Try to figure out the type of package using a file path
  /// to the package
  pub fn parse_from_dir(paths: &Paths, path: &Path) -> anyhow::Result<Self> {
    let mut path = path.to_path_buf();
    if path == paths.global {
      path = fs::read_link(&paths.global)?
    }
    let name_encoded = path.file_name().try_to_string()?;
    let name = name::decode(&name_encoded)?;

    let parent = path.try_parent()?;
    let parent_type = parent.file_name().try_to_string()?;

    PackageDescriptor::parse(
      paths,
      &VersionTarget::parse(format!("{}:{}", parent_type, name), paths)?,
    )
  }

  pub fn path_real(&self) -> anyhow::Result<PathBuf> {
    if self.path.is_symlink() {
      return Ok(fs::read_link(&self.path)?);
    }

    Ok(self.path.clone())
  }

  pub fn exists(&self) -> anyhow::Result<bool> {
    if self.path.is_symlink() {
      return Ok(true);
    }

    if self.path.is_dir() {
      return Ok(true);
    }

    if self.path.exists() {
      return Ok(true);
    }

    Ok(false)
  }
}
