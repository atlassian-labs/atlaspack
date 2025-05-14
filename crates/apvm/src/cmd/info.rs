use clap::Parser;

use crate::{
  context::Context,
  platform::package::{InstallablePackage, Package},
};

#[derive(Debug, Parser)]
pub struct InfoCommand {}

pub fn main(ctx: Context, _cmd: InfoCommand) -> anyhow::Result<()> {
  println!("Detected Version");
  if let Some(package) = ctx.versions.node_modules {
    match &package {
      Package::Local(_) => println!("  type: local"),
      Package::Npm(_) => println!("  type: npm"),
      Package::Git(_) => println!("  type: git"),
      Package::Unmanaged(_) => println!("  type: unmanaged"),
    };
    match &package {
      Package::Local(_) => {}
      Package::Npm(npm_package) => println!("  version: {}", npm_package.version),
      Package::Git(git_package) => println!("  version: {}", git_package.branch),
      Package::Unmanaged(_) => println!("  version: unknown"),
    };
  } else {
    println!("  <No version detected>");
  }

  println!("Detected Config");
  if let Some(apvmrc) = ctx.apvmrc {
  } else {
    println!("  <No config detected>");
  }

  println!();
  println!("Installed Versions");
  for version in &ctx.versions.installed {
    if let InstallablePackage::Npm(npm_package) = version {
      println!("  {}", npm_package.version)
    };
  }

  for version in &ctx.versions.installed {
    if let InstallablePackage::Git(git_package) = version {
      println!("  git:{}", git_package.branch)
    };
  }
  Ok(())
}
