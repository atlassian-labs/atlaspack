use clap::Parser;

use crate::context::Context;
use crate::platform::package::InstallablePackage;

#[derive(Debug, Parser)]
pub struct ListCommand {}

/// List the installed versions of Atlaspack available on the system
pub fn main(ctx: Context, _cmd: ListCommand) -> anyhow::Result<()> {
  for version in list_fmt(&ctx) {
    println!("* {}", version)
  }
  Ok(())
}

pub fn list_fmt(ctx: &Context) -> Vec<String> {
  let mut output = vec![];

  for version in &ctx.versions.installed {
    if let InstallablePackage::Npm(package) = version {
      output.push(format!("{}", package.version));
    };
  }

  for version in &ctx.versions.installed {
    if let InstallablePackage::Release(package) = version {
      output.push(format!("release:{}", package.version));
    };
  }

  for version in &ctx.versions.installed {
    if let InstallablePackage::Git(package) = version {
      output.push(format!("git:{}", package.version));
    };
  }

  output
}
