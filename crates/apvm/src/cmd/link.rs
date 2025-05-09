use clap::Parser;

use super::link_npm::npm_link_npm;
use crate::cmd::install::InstallCommand;
use crate::context::Context;
use crate::platform::origin::VersionTarget;
use crate::platform::package::PackageDescriptor;

#[derive(Debug, Parser, Clone)]
pub struct LinkCommand {
  /// Target version to link
  pub version: Option<String>,
}

pub fn main(ctx: Context, cmd: LinkCommand) -> anyhow::Result<()> {
  let version = VersionTarget::resolve(&ctx, &cmd.version)?;
  let package = PackageDescriptor::parse(&ctx.paths, &version)?;

  if !package.exists()?
    && !matches!(version, VersionTarget::Local(_))
    && !matches!(version, VersionTarget::LocalSuper(_))
  {
    super::install::main(
      ctx.clone(),
      InstallCommand {
        version: cmd.version.clone(),
        force: true,
        skip_build: false,
      },
    )?;
  };

  println!("Linking {}", package.version_target);

  match version {
    VersionTarget::Npm(_) => npm_link_npm(ctx, cmd, package)?,
    VersionTarget::Git(_) => npm_link_git(ctx, cmd, package)?,
    VersionTarget::Local(_) => npm_link_local(ctx, cmd, package)?,
    VersionTarget::LocalSuper(_) => npm_link_local_super(ctx, cmd, package)?,
  }

  println!("✅ Link completed");
  Ok(())
}

fn npm_link_git(
  _ctx: Context,
  _cmd: LinkCommand,
  _package: PackageDescriptor,
) -> anyhow::Result<()> {
  println!("TODO");
  Ok(())
}

fn npm_link_local(
  ctx: Context,
  cmd: LinkCommand,
  package: PackageDescriptor,
) -> anyhow::Result<()> {
  println!("Link Local");
  dbg!(&ctx);
  dbg!(&cmd);
  dbg!(&package);
  Ok(())
}

fn npm_link_local_super(
  ctx: Context,
  cmd: LinkCommand,
  package: PackageDescriptor,
) -> anyhow::Result<()> {
  println!("Link Local Super");
  dbg!(&ctx);
  dbg!(&cmd);
  dbg!(&package);
  Ok(())
}
