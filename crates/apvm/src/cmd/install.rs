use std::fs;
use std::time::SystemTime;

use clap::Parser;

use crate::cmd::install_git::install_from_git;
use crate::cmd::install_npm::install_from_npm;
use crate::context::Context;
use crate::platform::package::{InstallablePackage, ManagedPackage, Package};
use crate::platform::specifier::{self, Specifier};

#[derive(Debug)]
pub enum PackageResolveResult {
  Installed(ManagedPackage),
  NotInstalled(Specifier),
}

#[derive(Debug, Parser)]
pub struct InstallCommand {
  /// Target version to install
  #[clap(default_value = "default")]
  pub version: String,

  /// Replace an existing version if already installed
  #[arg(short = 'f', long = "force")]
  pub force: bool,

  /// Skips any build steps
  #[arg(long = "skip-build")]
  pub skip_build: bool,
}

pub fn main(ctx: Context, cmd: InstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  let specifier = ctx.resolver.resolve_specifier(&cmd.version)?;

  if let Some(package) = ctx.resolver.resolve(&specifier) {
    if !cmd.force {
      println!("✅ Already installed ({})", specifier);
      return Ok(());
    }
    println!("Removing Existing");
    match package {
      ManagedPackage::Local(_) => return Err(anyhow::anyhow!("Cannot delete local package")),
      ManagedPackage::Npm(npm_package) => fs::remove_dir_all(&npm_package.path)?,
      ManagedPackage::Git(git_package) => fs::remove_dir_all(&git_package.path)?,
    }
  }

  match &specifier {
    Specifier::Npm { version: _ } => install_from_npm(ctx, cmd, &specifier)?,
    Specifier::Git { branch } => todo!(),
    Specifier::Local => todo!(),
  };

  println!(
    "✅ Installed in {:.2?} ({})",
    start_time.elapsed()?,
    specifier
  );
  Ok(())
}
