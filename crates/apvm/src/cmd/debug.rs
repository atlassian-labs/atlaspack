// Hidden utilities to access dynamically set values
use clap::Parser;
use clap::Subcommand;

use crate::context::Context;

#[derive(Debug, Subcommand, Clone)]
pub enum DebugCommandType {}

#[derive(Debug, Parser)]
pub struct DebugCommand {
  #[clap(subcommand)]
  pub query: Option<DebugCommandType>,
}

pub fn main(ctx: Context, cmd: DebugCommand) -> anyhow::Result<()> {
  match cmd.query {
    None => {
      print!("{}", serde_json::to_string_pretty(&ctx)?);
    }
  }

  Ok(())
}
