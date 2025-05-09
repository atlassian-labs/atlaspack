use std::sync::mpsc::channel;

use super::exec::exec_blocking;
use super::exec::ExecOptions;
use super::runtime::resolve_runtime;
use crate::context::Context;
use crate::platform::origin::VersionTarget;
use crate::platform::package::PackageDescriptor;
use crate::platform::path_ext::*;

pub fn atlaspack_exec(mut ctx: Context) -> anyhow::Result<()> {
  let active_package = 'block: {
    let possible_version = ctx.env.argv.first().map(|v| v.as_str().to_string());
    if let Ok(version) = VersionTarget::resolve(&ctx, &possible_version) {
      let package = PackageDescriptor::parse(&ctx.paths, &version)?;
      if !package.exists()? {
        return Err(anyhow::anyhow!("Atlaspack version not installed"));
      }
      ctx.env.argv.remove(0);
      break 'block package;
    };

    if let Some(active) = &ctx.active_version {
      break 'block active.package.clone();
    };

    return Err(anyhow::anyhow!("No version of Atlaspack selected"));
  };

  log::info!("Running With: {:?}", active_package.path);

  let runtime = resolve_runtime(&ctx.env.runtime)?;

  let bin_path = match active_package.version_target {
    VersionTarget::Npm(_) => active_package.path.join("lib").join("cli.js"),
    VersionTarget::Git(_) => active_package
      .path
      .join("packages")
      .join("core")
      .join("cli")
      .join("lib")
      .join("cli.js"),
    VersionTarget::Local(_) => active_package
      .path
      .join("packages")
      .join("core")
      .join("cli")
      .join("lib")
      .join("cli.js"),
    VersionTarget::LocalSuper(_) => active_package
      .path
      .join("packages")
      .join("core")
      .join("cli")
      .join("lib")
      .join("cli.js"),
  };

  let mut args = Vec::<String>::new();

  args.push(runtime.try_to_string()?);
  args.push(bin_path.try_to_string()?);
  args.extend(ctx.env.argv);

  log::info!("{:?}", args);

  let (tx, rx) = channel::<anyhow::Result<()>>();

  // Run on separate thread to allow instant exit on cnt+c
  std::thread::spawn(move || {
    match exec_blocking(
      &args,
      ExecOptions {
        ..ExecOptions::default()
      },
    ) {
      Ok(_) => tx.send(Ok(())),
      Err(error) => tx.send(Err(error)),
    }
  });

  rx.recv()??;
  Ok(())
}
