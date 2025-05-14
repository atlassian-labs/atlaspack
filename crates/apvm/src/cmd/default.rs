use clap::Parser;

use crate::context::Context;

#[derive(Debug, Parser)]
pub struct DefaultCommand {
  /// Target version to use
  pub version: String,
}

pub fn main(_ctx: Context, _cmd: DefaultCommand) -> anyhow::Result<()> {
  // let version_target = VersionTarget::parse(&cmd.version, &ctx.paths)?;
  // let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;

  // if package.version == "local" || package.version == "local-super" {
  //   return Err(anyhow::anyhow!("TODO setting local as system default"));
  // }

  // if !package.exists()? {
  //   return Err(anyhow::anyhow!("Version not installed"));
  // }

  // if fs::exists(&ctx.paths.global)? {
  //   fs::remove_dir_all(&ctx.paths.global)?;
  // }

  // fs::create_dir_all(&ctx.paths.global)?;
  // link::soft_link(&package.path_real()?, &ctx.paths.global_atlaspack)?;

  // let version = if cmd.version == "local" {
  //   "local".to_string()
  // } else {
  //   version_target.to_string()
  // };
  // fs::write(&ctx.paths.global_version, version.as_bytes())?;
  // println!("✅ Default version set: {}", version_target);

  Ok(())
}
