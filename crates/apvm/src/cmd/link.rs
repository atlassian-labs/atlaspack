use std::time::SystemTime;

use clap::Parser;

use super::link_local::link_local;
use super::link_npm::link_npm;
use super::link_release::link_release;
use crate::context::Context;
use crate::platform::package::ManagedPackage;

#[derive(Debug, Parser, Clone)]
pub struct LinkCommand {
  /// Target version to link
  #[clap(default_value = "default")]
  pub version: String,

  /// Add version as an alias in the apvm.json
  #[clap(short = 'a', long = "set-alias", default_value = "default")]
  pub set_alias: Option<String>,
}

/// Link Atlaspack into the current project's node_modules
///
/// Order of version selection:
/// - apvmrc alias
/// - version specifier
pub fn main(ctx: Context, cmd: LinkCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();
  let specifier = ctx.resolver.resolve_specifier(&cmd.version)?;

  let package = match ctx.resolver.resolve(&specifier) {
    Some(package) => package,
    // Install package if not found
    None => {
      return Err(anyhow::anyhow!("Version not installed {}", specifier));
    }
  };

  // If there is a checksum specified,
  // check if it matches the incoming package
  if !ctx
    .validator
    .verify_package_checksum(&package.clone().into())?
  {
    return Err(anyhow::anyhow!(
      "Checksum of package does not match specified"
    ));
  }

  println!("Linking {:?}", cmd.version);
  match package {
    ManagedPackage::Local(package) => link_local(ctx, cmd, &specifier, package)?,
    ManagedPackage::Npm(package) => link_npm(ctx, cmd, &specifier, package)?,
    ManagedPackage::Git(_package) => link_git(ctx, cmd)?,
    ManagedPackage::Release(package) => link_release(ctx, cmd, &specifier, package)?,
  };

  println!("✅ Linked in {:.2?} ({})", start_time.elapsed()?, specifier);
  Ok(())
}

fn link_git(_ctx: Context, _cmd: LinkCommand) -> anyhow::Result<()> {
  println!("TODO");
  Ok(())
}
