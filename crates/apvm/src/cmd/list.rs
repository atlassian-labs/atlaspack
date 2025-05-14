use clap::Parser;

use crate::context::Context;
use crate::platform::package::InstallablePackage;

#[derive(Debug, Parser)]
pub struct ListCommand {}

pub fn main(ctx: Context, _cmd: ListCommand) -> anyhow::Result<()> {
  for version in &ctx.versions.installed {
    if let InstallablePackage::Npm(npm_package) = version {
      println!("* {}", npm_package.version)
    };
  }

  for version in &ctx.versions.installed {
    if let InstallablePackage::Git(git_package) = version {
      println!("* git:{}", git_package.branch)
    };
  }

  Ok(())
}
