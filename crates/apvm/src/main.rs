#![deny(unused_crate_dependencies)]

mod cmd;
mod context;
mod env;
mod paths;
mod platform;
mod public;
mod resolver;
mod validator;
mod versions;

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use env::Env;
use paths::Paths;
use platform::apvmrc::ApvmRc;
use resolver::PackageResolver;
use validator::Validator;
use versions::Versions;

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
  /// Print info on the current status
  Status(cmd::status::StatusCommand),
  /// Uninstall a previously installed version of Atlaspack
  Uninstall(cmd::uninstall::UninstallCommand),
  /// Version information
  Version,
  /// Run command with specified version of atlaspack
  Atlaspack(cmd::atlaspack::AtlaspackCommand),
  #[clap(hide = true)]
  Debug(cmd::debug::DebugCommand),
}

#[derive(Parser, Debug)]
pub struct ApvmCommand {
  #[clap(subcommand)]
  pub command: ApvmCommandType,
  /// Log all of the things
  #[arg(short = 'D', long = "debug")]
  pub debug: bool,
  /// [default value: "$HOME/.local/apvm"]
  #[arg(env = "APVM_DIR")]
  pub apvm_dir: Option<PathBuf>,
  // Path to local Atlaspack sources for local development
  #[arg(env = "APVM_ATLASPACK_LOCAL")]
  pub apvm_local: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
  // Checking args for the verbose flag before clap::parse()
  // to ensure logging starts before anything happens
  if std::env::args().any(|a| &a == "-D" || &a == "--debug") {
    unsafe { std::env::set_var("RUST_LOG", "debug") };
  }
  env_logger::init();

  log::info!("state_setup");
  let env = Env::parse()?;
  let paths = Paths::new(&env)?;
  let apvmrc = ApvmRc::new(&env.pwd)?;
  let versions = Versions::new(&paths, &apvmrc)?;
  let resolver = PackageResolver::new(&apvmrc, &versions);
  let validator = Validator::new(&apvmrc);

  let ctx = context::Context {
    versions,
    apvmrc,
    env,
    paths,
    resolver,
    validator,
  };

  // If the executable is called "atlaspack" then only proxy
  if &ctx.env.exe_stem == "atlaspack" {
    log::info!("Proxy mode");
    let cmd = cmd::atlaspack::AtlaspackCommand {
      version: None,
      argv: ctx.env.argv[1..].to_vec(),
    };
    return cmd::atlaspack::main(ctx, cmd);
  }

  log::info!("parsing_arguments");
  let args = ApvmCommand::parse_from(&ctx.env.argv);

  log::info!("running_command");
  match args.command {
    ApvmCommandType::Install(cmd) => cmd::install::main(ctx, cmd),
    ApvmCommandType::Status(cmd) => cmd::status::main(ctx, cmd),
    ApvmCommandType::Reinstall(cmd) => cmd::reinstall::main(ctx, cmd),
    ApvmCommandType::List(cmd) => cmd::list::main(ctx, cmd),
    ApvmCommandType::Uninstall(cmd) => cmd::uninstall::main(ctx, cmd),
    ApvmCommandType::Version => cmd::version::main(ctx),
    ApvmCommandType::Debug(cmd) => cmd::debug::main(ctx, cmd),
    ApvmCommandType::Default(cmd) => cmd::default::main(ctx, cmd),
    ApvmCommandType::Link(cmd) => cmd::link::main(ctx, cmd),
    ApvmCommandType::Atlaspack(cmd) => cmd::atlaspack::main(ctx, cmd),
  }
}
