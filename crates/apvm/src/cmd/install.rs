use std::fs;
use std::time::SystemTime;

use clap::Parser;

use crate::cmd::install_git::install_from_git;
use crate::cmd::install_npm::install_from_npm;
use crate::cmd::install_release::install_from_release;
use crate::context::Context;
use crate::platform::package::ManagedPackage;
use crate::platform::specifier::Specifier;

#[derive(Debug, Parser)]
pub struct InstallCommand {
  /// Target version to install
  #[clap(default_value = "default")]
  pub version: String,

  /// Replace an existing version if already installed
  #[arg(short = 'f', long = "force")]
  pub force: bool,

  /// Skips any build steps
  #[arg(long = "skip-postinstall")]
  pub skip_postinstall: bool,
}

/// Install a version of Atlaspack
///
/// Order of version selection:
/// - apvmrc alias
/// - version specifier
pub fn main(ctx: Context, cmd: InstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  let specifier = ctx.resolver.resolve_specifier(&cmd.version)?;
  println!("Installing ({})", specifier);

  if let Some(package) = ctx.resolver.resolve(&specifier)? {
    if !cmd.force {
      println!("✅ Already installed ({})", specifier);
      return Ok(());
    }
    println!("Removing Existing");
    match package {
      ManagedPackage::Local(_) => return Err(anyhow::anyhow!("Cannot delete local package")),
      ManagedPackage::Npm(pacakge) => fs::remove_dir_all(&pacakge.path)?,
      ManagedPackage::Git(pacakge) => fs::remove_dir_all(&pacakge.path)?,
      ManagedPackage::Release(pacakge) => fs::remove_dir_all(&pacakge.path)?,
    }
  }

  match &specifier {
    Specifier::Npm { version: _ } => install_from_npm(ctx, cmd, &specifier)?,
    Specifier::Git { version: _ } => install_from_git(ctx, cmd, &specifier)?,
    Specifier::Release { version: _ } => install_from_release(ctx, cmd, &specifier)?,
    Specifier::Local => unreachable!(),
  };

  println!(
    "✅ Installed in {:.2?} ({})",
    start_time.elapsed()?,
    specifier
  );
  Ok(())
}
