use std::fs;
use std::time::SystemTime;

use clap::Parser;

use super::install_local::install_from_local;
use crate::cmd::install_git::install_from_git;
use crate::cmd::install_npm::install_from_npm;
use crate::context::Context;
use crate::platform::origin::VersionTarget;
use crate::platform::package::PackageDescriptor;

#[derive(Debug, Parser)]
pub struct InstallCommand {
  /// Target version to install
  pub version: Option<String>,

  /// Replace an existing version if already installed
  #[arg(short = 'f', long = "force")]
  pub force: bool,

  /// Skips any build steps
  #[arg(long = "skip-build")]
  pub skip_build: bool,
}

pub fn main(ctx: Context, cmd: InstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  let version_target = VersionTarget::resolve(&ctx.apvmrc, &cmd.version)?;
  let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;
  let exists = package.exists()?;

  if exists && !cmd.force {
    println!("✅ Already installed ({})", version_target);
    return Ok(());
  }

  if exists {
    println!("Removing Existing");
    fs::remove_dir_all(&package.path)?;
  }

  match &version_target {
    VersionTarget::Npm(_) => install_from_npm(ctx, cmd, package)?,
    VersionTarget::Git(_) => install_from_git(ctx, cmd, package)?,
    VersionTarget::Local(_) => install_from_local(ctx, cmd, package)?,
  };

  println!(
    "✅ Installed in {:.2?} ({})",
    start_time.elapsed()?,
    version_target
  );
  Ok(())
}
