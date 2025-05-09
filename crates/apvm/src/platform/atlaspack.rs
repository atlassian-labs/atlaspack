use std::sync::mpsc::channel;

use super::exec::exec_blocking;
use super::exec::ExecOptions;
use super::runtime::resolve_runtime;
use crate::context::Context;
use crate::platform::origin::VersionTarget;
use crate::platform::path_ext::*;

pub fn atlaspack_exec(ctx: &Context, command: Vec<String>) -> anyhow::Result<()> {
  let runtime = resolve_runtime(&ctx.env.runtime)?;

  let Some(active) = &ctx.active_version else {
    return Err(anyhow::anyhow!("No version of Atlaspack selected"));
  };

  let bin_path = match active.package.version_target {
    VersionTarget::Npm(_) => active.package.path.join("lib").join("cli.js"),
    VersionTarget::Git(_) => active
      .package
      .path
      .join("packages")
      .join("core")
      .join("cli")
      .join("lib")
      .join("cli.js"),
    VersionTarget::Local(_) => active
      .package
      .path
      .join("packages")
      .join("core")
      .join("cli")
      .join("lib")
      .join("cli.js"),
    VersionTarget::LocalSuper(_) => active
      .package
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
  args.extend(command);

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
