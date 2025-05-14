use clap::Parser;

use super::link_local::link_local;
use super::link_npm::link_npm;
use crate::cmd::install::InstallCommand;
use crate::context::Context;
use crate::platform::package::ManagedPackage;

#[derive(Debug, Parser, Clone)]
pub struct LinkCommand {
  /// Target version to link
  #[clap(default_value = "default")]
  pub version: String,
}

pub fn main(ctx: Context, cmd: LinkCommand) -> anyhow::Result<()> {
  let specifier = ctx.resolver.resolve_specifier(&cmd.version)?;

  let package = match ctx.resolver.resolve(&specifier) {
    Some(package) => package,
    // Install package if not found
    None => {
      super::install::main(
        ctx.clone(),
        InstallCommand {
          version: cmd.version.clone(),
          force: true,
          skip_build: false,
        },
      )?;
      ctx.resolver.resolve(&specifier).unwrap()
    }
  };

  println!("Linking {:?}", cmd.version);
  match package {
    ManagedPackage::Local(local_package) => link_local(ctx, cmd, &specifier, local_package)?,
    ManagedPackage::Npm(npm_package) => link_npm(ctx, cmd, &specifier, npm_package)?,
    ManagedPackage::Git(_git_package) => npm_link_git(ctx, cmd)?,
  };

  println!("✅ Link completed");
  Ok(())
}

fn npm_link_git(_ctx: Context, _cmd: LinkCommand) -> anyhow::Result<()> {
  println!("TODO");
  Ok(())
}
