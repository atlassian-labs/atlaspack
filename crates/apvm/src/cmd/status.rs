use clap::Parser;

use crate::context::Context;
use crate::platform::colors::*;
use crate::platform::package::Package;
use crate::platform::path_ext::PathExt;

#[derive(Debug, Parser)]
pub struct StatusCommand {}

/// Print human readable information on the apvm state / what apvm sees
pub fn main(ctx: Context, _cmd: StatusCommand) -> anyhow::Result<()> {
  println!("{style_underline}Detected Version{style_reset}");
  if let Some(package) = ctx.versions.node_modules()? {
    match &package {
      Package::Local(_) => println!("  Type: local"),
      Package::Npm(_) => println!("  Type: npm"),
      Package::Git(_) => println!("  Type: git"),
      Package::Release(_) => println!("  Type: release"),
      Package::Unmanaged(_) => println!("  Type: unmanaged"),
      Package::Default(_) => {}
    };
    match &package {
      Package::Local(_) => {}
      Package::Npm(package) => println!("  Version: {}", package.version),
      Package::Git(package) => println!("  Version: {}", package.version),
      Package::Release(package) => println!("  Version: {}", package.version),
      Package::Unmanaged(_) => println!("  Version: unknown"),
      Package::Default(_) => {}
    };

    println!("  Location: node_modules");
  } else {
    println!("  <No version detected>");
  }

  println!();
  println!("{style_underline}Detected Config{style_reset}");
  if ctx.apvmrc.exists {
    println!("  Path: {}", ctx.apvmrc.path.try_to_string()?);

    println!();
    println!("{style_underline}Config Aliases{style_reset}");
    for (alias, version) in ctx.apvmrc.get_aliases() {
      println!("  \"{}\" -> \"{}\"", alias, version);
    }
  } else {
    println!("  <No config detected>");
  }

  println!();
  println!("{style_underline}Installed Versions{style_reset}");

  for version in crate::cmd::list::list_fmt(&ctx)? {
    println!("  {}", version)
  }
  Ok(())
}
