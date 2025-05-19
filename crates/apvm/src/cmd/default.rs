use std::fs;

use clap::Parser;

use crate::context::Context;
use crate::platform::fs_ext;
use crate::platform::package::DefaultPackage;
use crate::platform::package::ManagedPackage;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;
use crate::public::package_kind::PackageKind;

#[derive(Debug, Parser)]
pub struct DefaultCommand {
  /// Target version to use
  pub version: String,
}

/// Sets the global/fallback version of Atlaspack
pub fn main(ctx: Context, cmd: DefaultCommand) -> anyhow::Result<()> {
  let specifier = ctx.resolver.resolve_specifier(&cmd.version)?;

  let package = match ctx.resolver.resolve(&specifier)? {
    Some(package) => package,
    None => {
      return Err(anyhow::anyhow!("Version not installed"));
    }
  };

  println!("Setting global version to {:?}", cmd.version);

  let meta = match &package {
    ManagedPackage::Local(package) => LinkedMeta {
      kind: PackageKind::Local,
      version: Some(specifier.version().to_string()),
      specifier: Some(specifier.to_string()),
      original_path: package.path.clone(),
      content_path: package.path.clone(),
    },
    ManagedPackage::Npm(package) => LinkedMeta {
      kind: PackageKind::Npm,
      version: Some(specifier.version().to_string()),
      specifier: Some(specifier.to_string()),
      original_path: package.path.clone(),
      content_path: package.contents(),
    },
    ManagedPackage::Git(package) => LinkedMeta {
      kind: PackageKind::Git,
      version: Some(specifier.version().to_string()),
      specifier: Some(specifier.to_string()),
      original_path: package.path.clone(),
      content_path: package.contents(),
    },
    ManagedPackage::Release(package) => LinkedMeta {
      kind: PackageKind::Release,
      version: Some(specifier.version().to_string()),
      specifier: Some(specifier.to_string()),
      original_path: package.path.clone(),
      content_path: package.contents(),
    },
  };

  let default = DefaultPackage::new(&ctx.paths.global, &meta);

  if fs::exists(&default.path)? {
    fs::remove_dir_all(&default.path)?;
  }
  fs::create_dir_all(&default.path)?;

  fs_ext::soft_link(&default.meta.original_path, &default.link_path)?;
  meta.write_to_file(default.meta())?;

  println!("✅ Default version set ({})", specifier);

  Ok(())
}
