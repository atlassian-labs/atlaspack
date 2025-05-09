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
  Atlaspack {
    /// Version of Atlaspack to run command with
    version: Option<String>,

    /// Arguments to pass to Atlaspack
    #[arg(last(true))]
    args: Vec<String>,
  },
  #[clap(hide = true)]
  Debug(cmd::debug::DebugCommand),
}

#[derive(Parser, Debug)]
pub struct ApvmCommand {
  #[clap(subcommand)]
  pub command: ApvmCommandType,
  /// Log all of the things
  #[arg(short = 'v', long = "verbose")]
  pub verbose: bool,
  /// [default value: "$HOME/.local/apvm"]
  #[arg(env = "APVM_DIR")]
  pub apvm_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
  // Checking args for the verbose flag before clap::parse() to ensure
  // logging starts before anything happens
  if std::env::args().any(|a| &a == "-v" || &a == "--verbose") {
    std::env::set_var("RUST_LOG", "debug");
  }
  env_logger::init();

  let env = Env::parse()?;
  let paths = Paths::new(&env)?;
  let apvmrc = ApvmRc::detect(&env.pwd, &paths)?;
  let active_version = ActiveVersion::detect(&apvmrc, &paths)?;
  let ctx = context::Context {
    env,
    paths,
    apvmrc,
    active_version,
  };

  // If the executable is called "atlaspack" then only proxy
  if &ctx.env.exe_stem == "atlaspack" {
    let mut args = ctx.env.argv.clone();
    args.remove(0);
    return atlaspack_exec(ctx, args, None);
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
    ApvmCommandType::Atlaspack { version, args } => atlaspack_exec(ctx, args, version),
  }
}

/*
    // Remove the first arg and forward remaining to the exec proxy
    // The arg structure is [arg0, command, ...args]
    // This forwards only "args" to the exec proxy
    let mut ctx = ctx;
    ctx.env.argv.remove(0);
    return atlaspack_exec(&ctx, ctx.env.argv.clone());

*/
