use clap::Parser;

use crate::context::Context;
use crate::platform::bin_path::get_bin_path;
use crate::platform::exec::exec_blocking;
use crate::platform::exec::ExecOptions;
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

/// Proxy commands to the detected Atlaspack version's CLI entry point
///
/// Order of version selection:
/// - version specifier
/// - node_modules bin
/// - apvmrc alias
/// - system default
pub fn main(ctx: Context, cmd: AtlaspackCommand) -> anyhow::Result<()> {
  let package = ctx.resolver.resolve_or_default(&cmd.version)?;
  let bin_path = get_bin_path(&package);

  let runtime = resolve_runtime(&ctx.env.runtime)?.try_to_string()?;
  log::info!("runtime: {:?}", runtime);

  let mut argv = Vec::<String>::new();

  argv.push(runtime.clone());
  if runtime.ends_with("deno") {
    argv.push("--allow-all".to_string());
  }

  log::info!("binary_path: {:?}", bin_path);
  argv.push(bin_path.try_to_string()?);

  log::info!("atlaspack_args: {:?}", cmd.argv);
  argv.extend(cmd.argv);

  let options = ExecOptions {
    stdio: true,
    ..ExecOptions::default()
  };

  log::info!("args: {:?}", argv);
  match std::thread::spawn(move || exec_blocking(&argv, &options)).join() {
    Ok(result) => result,
    Err(err) => Err(anyhow::anyhow!("Thread encountered error: {:?}", err)),
  }
}
