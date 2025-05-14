use std::fs;
use std::time::SystemTime;

use clap::Parser;

use crate::context::Context;

#[derive(Debug, Parser)]
pub struct UninstallCommand {
  /// Target version to uninstall
  pub version: String,
}

pub fn main(ctx: Context, cmd: UninstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  // let version_target = VersionTarget::resolve(&ctx, &Some(cmd.version.clone()))?;
  // let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;

  // let pkg = ctx.paths.versions_v1_pkg(&package.version_encoded);
  // if !fs::exists(&pkg.base)? {
  //   return Err(anyhow::anyhow!("Not installed",));
  // }

  // println!("Removing {}", cmd.version);
  // fs::remove_dir_all(&pkg.base)?;

  // println!(
  //   "✅ Uninstalled in {:.2?} ({})",
  //   start_time.elapsed()?,
  //   cmd.version
  // );

  Ok(())
}
