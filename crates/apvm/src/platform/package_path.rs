use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PackagePath {
  /// A package installed to $APVM_DIR
  Installed(PackagePathInstalled),
  /// A reference to local Atlaspack sources
  Local(PackagePathLocal),
  /// A pre-existing installation of Atlaspack in node_modules
  Unmanaged(PackagePathUnmanaged),
}

impl PackagePath {
  pub fn installed_from_path<P: AsRef<Path>>(target: P) -> Self {
    let target = target.as_ref();
    Self::Installed(PackagePathInstalled {
      meta: target.join("meta.json"),
      contents: target.join("contents"),
      base: target.to_path_buf(),
    })
  }

  pub fn local_from_path<P: AsRef<Path>>(target: P) -> Self {
    Self::Local(PackagePathLocal {
      base: target.as_ref().to_path_buf(),
    })
  }

  pub fn exists(&self) -> std::io::Result<bool> {
    fs::exists(match self {
      PackagePath::Installed(pkg) => &pkg.base,
      PackagePath::Local(pkg) => &pkg.base,
      PackagePath::Unmanaged(pkg) => &pkg.base,
    })
  }

  pub fn as_installed(&self) -> &PackagePathInstalled {
    let PackagePath::Installed(pkg) = self else {
      unreachable!()
    };
    pkg
  }

  pub fn as_local(&self) -> &PackagePathLocal {
    let PackagePath::Local(pkg) = self else {
      unreachable!()
    };
    pkg
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePathInstalled {
  /// $APVM_DIR/versions_v1/<name>
  pub base: PathBuf,
  /// $APVM_DIR/versions_v1/<name>/meta.json
  pub meta: PathBuf,
  /// $APVM_DIR/versions_v1/<name>/contents
  pub contents: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePathLocal {
  /// $APVM_ATLASPACK_LOCAL
  pub base: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePathUnmanaged {
  /// $PWD/node_modules/@atlaspack
  pub base: PathBuf,
}
