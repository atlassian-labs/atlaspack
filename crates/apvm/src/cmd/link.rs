use std::time::SystemTime;

use clap::Parser;

use super::link_local::link_local;
use super::link_npm::link_npm;
use super::link_release::link_release;
use crate::cmd::install::InstallCommand;
use crate::cmd::link_git::link_git;
use crate::context::Context;
use crate::platform::package::ManagedPackage;

#[derive(Debug, Parser, Clone)]
pub struct LinkCommand {
  /// Target version to link
  #[clap(default_value = "default")]
  pub version: String,

  /// Add version as an alias in the apvm.json
  #[clap(long = "set-alias")]
  pub set_alias: Option<String>,

  #[clap(long = "strict")]
  pub strict: bool,
}

/// Link Atlaspack into the current project's node_modules
///
/// Order of version selection:
/// - apvmrc alias
/// - version specifier
pub fn main(ctx: Context, cmd: LinkCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();
  let specifier = ctx.resolver.resolve_specifier(&cmd.version)?;

  let package = match ctx.resolver.resolve(&specifier)? {
    Some(package) => package,
    // Install package if not found
    None => {
      println!("Not Installed");
      crate::cmd::install::main(
        ctx.clone(),
        InstallCommand {
          version: cmd.version.clone(),
          force: true,
          skip_postinstall: false,
        },
      )?;
      let Some(package) = ctx.resolver.resolve(&specifier)? else {
        return Err(anyhow::anyhow!(
          "Package installed but unable to link, please try again"
        ));
      };
      package
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

  if cmd.strict && ctx.apvmrc.get_checksum(&specifier).is_none() {
    return Err(anyhow::anyhow!(
      "Cannot link package without checksum when in strict mode"
    ));
  }

  println!("Linking {:?}", cmd.version);
  match package {
    ManagedPackage::Local(package) => link_local(ctx, cmd, &specifier, package)?,
    ManagedPackage::Npm(package) => link_npm(ctx, cmd, &specifier, package)?,
    ManagedPackage::Git(package) => link_git(ctx, cmd, &specifier, package)?,
    ManagedPackage::Release(package) => link_release(ctx, cmd, &specifier, package)?,
  };

  println!("✅ Linked in {:.2?} ({})", start_time.elapsed()?, specifier);
  Ok(())
}
