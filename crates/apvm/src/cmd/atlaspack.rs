use clap::Parser;

use crate::context::Context;
use crate::platform::exec::exec_blocking;
use crate::platform::exec::ExecOptions;
use crate::platform::origin::VersionTarget;
use crate::platform::package::PackageDescriptor;
use crate::platform::path_ext::PathExt;
use crate::platform::runtime::resolve_runtime;

#[derive(Debug, Parser)]
pub struct AtlaspackCommand {
  /// Arguments to pass into Atlaspack
  #[arg(last = true)]
  pub argv: Vec<String>,
  /// Replace an existing version if already installed
  #[arg(short = 'V', long = "version")]
  pub version: Option<String>,
}

pub fn main(ctx: Context, cmd: AtlaspackCommand) -> anyhow::Result<()> {
  let bin_path = if let Some(version) = &cmd.version {
    let version_target = VersionTarget::resolve(&ctx, &Some(version.clone()))?;
    let package = PackageDescriptor::parse(&ctx.paths, &version_target)?;
    if !package.exists()? {
      return Err(anyhow::anyhow!("Atlaspack version not installed"));
    }
    package.bin_path
  } else if let Some(active_version) = ctx.active_version {
    active_version.bin_path
  } else {
    return Err(anyhow::anyhow!("No Atlaspack version installed"));
  };

  log::info!("Binary Path {:?}", bin_path);
  log::info!("Argv {:?}", cmd.argv);

  let mut argv = Vec::<String>::new();
  argv.push(resolve_runtime(&ctx.env.runtime)?.try_to_string()?);
  argv.push(bin_path.try_to_string()?);
  argv.extend(cmd.argv);

  match std::thread::spawn(move || exec_blocking(&argv, ExecOptions::default())).join() {
    Ok(result) => result,
    Err(err) => Err(anyhow::anyhow!("Thread encountered error: {:?}", err)),
  }
}
