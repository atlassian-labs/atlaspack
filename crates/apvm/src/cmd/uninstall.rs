use std::fs;
use std::time::SystemTime;

use clap::Parser;

use crate::cmd::install_npm::resolve_from_npm;
use crate::context::Context;
use crate::platform::package::ManagedPackage;
use crate::platform::specifier::Specifier;

#[derive(Debug, Parser)]
pub struct UninstallCommand {
  /// Target version to uninstall
  pub version: String,
}

/// Uninstall a version of Atlaspack
pub fn main(ctx: Context, cmd: UninstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  let specifier_raw = ctx.resolver.resolve_specifier(&cmd.version)?;

  let specifier = match &specifier_raw {
    Specifier::Npm { version } => Specifier::Npm {
      version: resolve_from_npm(&ctx, version)?,
    },
    Specifier::Git { version: _ } => specifier_raw,
    Specifier::Release { version: _ } => specifier_raw,
    Specifier::Local => specifier_raw,
  };

  if let Some(package) = ctx.resolver.resolve(&specifier)? {
    let path = match &package {
      ManagedPackage::Local(_package) => {
        return Err(anyhow::anyhow!("Cannot uninstall local"));
      }
      ManagedPackage::Npm(package) => &package.path,
      ManagedPackage::Release(package) => &package.path,
      ManagedPackage::Git(package) => &package.path,
    };

    if !fs::exists(path)? {
      return Err(anyhow::anyhow!("Not installed",));
    }

    println!("Removing {}", specifier);
    fs::remove_dir_all(path)?;
  }

  println!(
    "✅ Uninstalled in {:.2?} ({})",
    start_time.elapsed()?,
    cmd.version
  );

  Ok(())
}
