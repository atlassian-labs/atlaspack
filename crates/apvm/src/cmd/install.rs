use std::fs;
use std::time::SystemTime;

use clap::Parser;

use crate::cmd::install_git::install_from_git;
use crate::cmd::install_npm::install_from_npm;
use crate::context::Context;

#[derive(Debug, Parser)]
pub struct InstallCommand {
  /// Target version to install
  pub version: Option<String>,

  /// Replace an existing version if already installed
  #[arg(short = 'f', long = "force")]
  pub force: bool,

  /// Skips any build steps
  #[arg(long = "skip-build")]
  pub skip_build: bool,
}

pub fn main(ctx: Context, cmd: InstallCommand) -> anyhow::Result<()> {
  let start_time = SystemTime::now();

  // let (kind, specifier) = match ctx.versions.resolve(&cmd.version)? {
  //   VersionResolveResult::Invalid => return Err(anyhow::anyhow!("Invalid install request")),
  //   VersionResolveResult::CannotResolve => {
  //     return Err(anyhow::anyhow!("Failed to resolve version specifier"))
  //   }
  //   VersionResolveResult::Installed(version) => {
  //     if !cmd.force {
  //       println!(
  //         "✅ Already installed ({})",
  //         version.specifier.unwrap_or_default()
  //       );
  //       return Ok(());
  //     }
  //     (version.kind, version.specifier)
  //   }
  //   VersionResolveResult::NotInstalledLocal => {
  //     return Err(anyhow::anyhow!("Local version not defined"))
  //   }
  //   VersionResolveResult::NotInstalled { specifier, kind } => (kind, specifier),
  // };

  // let Some(specifier) = specifier else {
  //   return Err(anyhow::anyhow!("No specifier provided"));
  // };

  // dbg!(&kind);
  // dbg!(&specifier);

  // let (kind, specifier) = match ctx.versions.resolve(&cmd.version)? {
  //   Some(VersionResolveResult::Installed(version)) => match version.kind {
  //     PackageKind::Local => (PackageKind::Local, "".to_string()),
  //     PackageKind::Unmanaged => return Err(anyhow::anyhow!("Cannot reinstall unmanaged")),
  //     _ => {
  //       let Some(specifier) = version.specifier else {
  //         return Err(anyhow::anyhow!("No specifier provided"));
  //       };

  //       if !cmd.force {
  //         println!("✅ Already installed ({:?})", specifier);
  //         return Ok(());
  //       }

  //       (version.kind, specifier)
  //     }
  //   },
  //   Some(VersionResolveResult::NotInstalled { specifier, kind }) => (kind, specifier),
  //   None => {
  //     let Some(specifier) = cmd.version else {
  //       return Err(anyhow::anyhow!("No specifier provided"));
  //     };
  //     if specifier == "local" {
  //       return Err(anyhow::anyhow!("No local version installed"));
  //     }
  //     let (kind, specifier) = parse_specifier(&specifier)?;
  //     let Some(specifier) = specifier else {
  //       return Err(anyhow::anyhow!("No specifier provided"));
  //     };
  //     (kind, specifier)
  //   }
  // };

  // dbg!(&kind);
  // dbg!(&specifier);
  // if let Some(version) = ctx.versions.resolve(&cmd.version)? {};
  // let version_target = VersionTarget::resolve(&ctx, &cmd.version)?;
  // let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;
  // let exists = package.exists()?;

  // if exists && !cmd.force {
  //   println!("✅ Already installed ({})", version_target);
  //   return Ok(());
  // }

  // if exists {
  //   println!("Removing Existing");
  //   fs::remove_dir_all(&package.path)?;
  // }

  // match &version_target {
  //   VersionTarget::Npm(_) => install_from_npm(ctx, cmd, package)?,
  //   VersionTarget::Git(_) => install_from_git(ctx, cmd, package)?,
  //   VersionTarget::Local(_) => {
  //     return Err(anyhow::anyhow!("Cannot install a local version"));
  //   }
  //   VersionTarget::LocalSuper(_) => {
  //     return Err(anyhow::anyhow!("Cannot install a local version"));
  //   }
  // };

  // println!(
  //   "✅ Installed in {:.2?} ({})",
  //   start_time.elapsed()?,
  //   version_target
  // );
  Ok(())
}
