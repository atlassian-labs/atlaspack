use std::path::{Path, PathBuf};

use crate::public::package_kind::PackageKind;

use super::package::Package;

pub fn get_bin_path(input: &Package) -> PathBuf {
  match input {
    Package::Local(local_package) => repo_bin_path(&local_package.path),
    Package::Npm(npm_package) => npm_bin_path(npm_package.contents()),
    Package::Git(git_package) => repo_bin_path(&git_package.path),
    Package::Release(release_package) => release_bin_path(release_package.contents()),
    Package::Unmanaged(unmanaged_package) => unmanaged_bin_path(&unmanaged_package.path),
    Package::Default(default_package) => match default_package.meta.kind {
      PackageKind::Npm => npm_bin_path(&default_package.meta.content_path),
      PackageKind::Release => release_bin_path(&default_package.meta.content_path),
      PackageKind::Git => repo_bin_path(&default_package.meta.content_path),
      PackageKind::Local => repo_bin_path(&default_package.meta.content_path),
      PackageKind::Unmanaged => unmanaged_bin_path(&default_package.meta.content_path),
    },
  }
}

pub fn repo_bin_path<P: AsRef<Path>>(base: P) -> PathBuf {
  base
    .as_ref()
    .join("packages")
    .join("core")
    .join("cli")
    .join("lib")
    .join("bin.js")
}

pub fn release_bin_path<P: AsRef<Path>>(base: P) -> PathBuf {
  base.as_ref().join("cmd").join("main.js")
}

pub fn npm_bin_path<P: AsRef<Path>>(base: P) -> PathBuf {
  base.as_ref().join("static").join("cli").join("index.js")
}

pub fn unmanaged_bin_path<P: AsRef<Path>>(base: P) -> PathBuf {
  base.as_ref().join("cli").join("lib").join("bin.js")
}
