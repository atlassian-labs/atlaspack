use std::fs;

use clap::Parser;

use crate::context::Context;
use crate::platform::link;
use crate::platform::origin::VersionTarget;
use crate::platform::package::PackageDescriptor;

#[derive(Debug, Parser)]
pub struct DefaultCommand {
  /// Target version to use
  pub version: String,
}

pub fn main(ctx: Context, cmd: DefaultCommand) -> anyhow::Result<()> {
  let version_target = VersionTarget::parse(&cmd.version)?;
  let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;

  if !package.exists()? {
    return Err(anyhow::anyhow!("Version not installed"));
  }

  if fs::exists(&ctx.paths.global)? {
    fs::remove_dir_all(&ctx.paths.global)?;
  }

  link::soft_link(&package.path_real()?, &ctx.paths.global)?;

  println!("✅ Default version set: {}", version_target);

  Ok(())
}
