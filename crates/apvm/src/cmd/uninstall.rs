use std::fs;
use std::time::SystemTime;

use clap::Parser;

use crate::context::Context;
use crate::platform::origin::VersionTarget;
use crate::platform::package::PackageDescriptor;

#[derive(Debug, Parser)]
pub struct UninstallCommand {
  /// Target version to uninstall
  pub version: String,
}

pub fn main(ctx: Context, cmd: UninstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  let version_target = VersionTarget::parse(&cmd.version, &ctx.paths)?;
  let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;

  if !package.exists()? {
    return Err(anyhow::anyhow!("Not installed",));
  }

  println!("Removing {}", cmd.version);
  fs::remove_dir_all(package.path)?;

  println!(
    "✅ Uninstalled in {:.2?} ({})",
    start_time.elapsed()?,
    cmd.version
  );

  Ok(())
}
