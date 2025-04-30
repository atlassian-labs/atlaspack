#![deny(unused_crate_dependencies)]

mod cmd;
mod context;
mod env;
mod paths;
mod platform;

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use env::Env;
use paths::Paths;
use platform::active::ActiveVersion;
use platform::apvmrc::ApvmRc;
use platform::atlaspack::atlaspack_exec;

#[derive(Debug, Subcommand)]
pub enum ApvmCommandType {
  /// Set the default version of Atlaspack
  Default(cmd::default::DefaultCommand),
  /// Install a version of Atlaspack
  Install(cmd::install::InstallCommand),
  /// Helpers to work with node_modules
  Link(cmd::link::LinkCommand),
  /// List installed versions of Atlaspack
  List(cmd::list::ListCommand),
  /// Reinstall a previously installed version of Atlaspack
  Reinstall(cmd::install::InstallCommand),
  /// Uninstall a previously installed version of Atlaspack
  Uninstall(cmd::uninstall::UninstallCommand),
  /// Version information
  Version,
  /// Run command with specified version of atlaspack
  Atlaspack,
  #[clap(hide = true)]
  Debug(cmd::debug::DebugCommand),
}

#[derive(Parser, Debug)]
pub struct ApvmCommand {
  #[clap(subcommand)]
  pub command: ApvmCommandType,
  /// [default value: "$HOME/.local/apvm"]
  #[arg(env = "APVM_DIR")]
  pub apvm_dir: Option<PathBuf>,
  /// [possible values: "error", "warn", "info", "debug", "trace"]
  #[arg(env = "RUST_LOG")]
  pub _rust_log: Option<String>,
}

fn main() -> anyhow::Result<()> {
  env_logger::init();

  let env = Env::parse()?;
  let paths = Paths::new(&env)?;
  let apvmrc = ApvmRc::detect(&env.pwd)?;
  let active_version = ActiveVersion::detect(&paths)?;
  let ctx = context::Context {
    env,
    paths,
    apvmrc,
    active_version,
  };

  // If the executable is called "atlaspack" then only proxy
  if &ctx.env.exe_stem == "atlaspack" {
    return atlaspack_exec(&ctx, ctx.env.argv.clone());
  }

  // Calling "apvm atlaspack" will proxy to the active Atlaspack version
  if let Some("atlaspack") = ctx.env.argv.first().map(|v| v.as_str()) {
    // Remove the first arg and forward remaining to the exec proxy
    // The arg structure is [arg0, command, ...args]
    // This forwards only "args" to the exec proxy
    let mut ctx = ctx;
    ctx.env.argv.remove(0);
    return atlaspack_exec(&ctx, ctx.env.argv.clone());
  }

  // APVM Commands
  let args = ApvmCommand::parse();

  match args.command {
    ApvmCommandType::Install(cmd) => cmd::install::main(ctx, cmd),
    ApvmCommandType::Reinstall(cmd) => cmd::reinstall::main(ctx, cmd),
    ApvmCommandType::List(cmd) => cmd::list::main(ctx, cmd),
    ApvmCommandType::Uninstall(cmd) => cmd::uninstall::main(ctx, cmd),
    ApvmCommandType::Version => cmd::version::main(ctx),
    ApvmCommandType::Debug(cmd) => cmd::debug::main(ctx, cmd),
    ApvmCommandType::Default(cmd) => cmd::default::main(ctx, cmd),
    ApvmCommandType::Link(cmd) => cmd::link::main(ctx, cmd),
    ApvmCommandType::Atlaspack => unreachable!(),
  }
}
