use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::encoder;
use super::specifier::Specifier;
use crate::platform::constants as c;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;

/// All Package types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Package {
  Local(LocalPackage),
  Npm(NpmPackage),
  Git(GitPackage),
  Release(ReleasePackage),
  Unmanaged(UnmanagedPackage),
  Default(DefaultPackage),
}

impl From<ManagedPackage> for Package {
  fn from(value: ManagedPackage) -> Self {
    match value {
      ManagedPackage::Local(package) => Self::Local(package),
      ManagedPackage::Npm(package) => Self::Npm(package),
      ManagedPackage::Git(package) => Self::Git(package),
      ManagedPackage::Release(package) => Self::Release(package),
    }
  }
}

/// Package types that can be linked into node_modules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManagedPackage {
  Local(LocalPackage),
  Npm(NpmPackage),
  Git(GitPackage),
  Release(ReleasePackage),
}

/// Package types that can be installed from remote
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallablePackage {
  Npm(NpmPackage),
  Git(GitPackage),
  Release(ReleasePackage),
}

/// Represents apvm managed Atlaspack installed from local sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalPackage {
  /// $APVM_ATLASPACK_LOCAL
  pub path: PathBuf,
}

/// Represents apvm managed Atlaspack installed from npm (super)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NpmPackage {
  pub specifier: String,
  pub version: String,
  /// $APVM_DIR/versions_v1/<name>
  pub path: PathBuf,
  pub alias: Option<String>,
  pub checksum: Option<String>,
}

impl NpmPackage {
  pub fn from_name(base: &Path, version: &Specifier) -> anyhow::Result<Self> {
    Ok(Self {
      specifier: version.to_string(),
      version: version.version().to_string(),
      path: base.join(encoder::encode(version.to_string())?),
      alias: None,
      checksum: None,
    })
  }

  pub fn meta_file(&self) -> PathBuf {
    self.path.join(c::PACKAGE_META_FILE)
  }

  pub fn contents(&self) -> PathBuf {
    self.path.join("contents")
  }
}

/// Represents apvm managed Atlaspack installed from git
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GitPackage {
  pub specifier: String,
  pub version: String,
  /// $APVM_DIR/versions_v1/<name>
  pub path: PathBuf,
  pub alias: Option<String>,
}

impl GitPackage {
  pub fn from_name(base: &Path, version: &Specifier) -> anyhow::Result<Self> {
    Ok(Self {
      specifier: version.to_string(),
      version: version.version().to_string(),
      path: base.join(encoder::encode(version.to_string())?),
      alias: None,
    })
  }

  pub fn meta_file(&self) -> PathBuf {
    self.path.join(c::PACKAGE_META_FILE)
  }

  pub fn contents(&self) -> PathBuf {
    self.path.join("contents")
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleasePackage {
  pub specifier: String,
  pub version: String,
  /// $APVM_DIR/versions_v1/<name>
  pub path: PathBuf,
  pub alias: Option<String>,
  pub checksum: Option<String>,
}

impl ReleasePackage {
  pub fn from_name(base: &Path, version: &Specifier) -> anyhow::Result<Self> {
    Ok(Self {
      specifier: version.to_string(),
      version: version.version().to_string(),
      path: base.join(encoder::encode(version.to_string())?),
      alias: None,
      checksum: None,
    })
  }

  pub fn meta_file(&self) -> PathBuf {
    self.path.join(c::PACKAGE_META_FILE)
  }

  pub fn contents(&self) -> PathBuf {
    self.path.join("contents")
  }
}

/// Represents Atlaspack installed into node_modules without apvm
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UnmanagedPackage {
  /// $PWD/node_modules/@atlaspack
  pub path: PathBuf,
}

/// Represents a package linked globally
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DefaultPackage {
  /// $APVM_DIR/global
  pub path: PathBuf,
  /// $APVM_DIR/global/package
  pub link_path: PathBuf,
  pub meta: LinkedMeta,
}

impl DefaultPackage {
  pub fn new_from_dir(base: &Path) -> anyhow::Result<Self> {
    let linked_meta = LinkedMeta::read_from_file(base.join(c::LINK_META_FILE))?;
    Ok(Self::new(base, &linked_meta))
  }

  pub fn new(base: &Path, linked_meta: &LinkedMeta) -> Self {
    Self {
      path: base.to_path_buf(),
      link_path: base.join("package"),
      meta: linked_meta.clone(),
    }
  }

  pub fn meta(&self) -> PathBuf {
    self.path.join(c::LINK_META_FILE)
  }
}
