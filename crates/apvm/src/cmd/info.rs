use clap::Parser;

use crate::context::Context;

#[derive(Debug, Parser)]
pub struct InfoCommand {}

pub fn main(_ctx: Context, _cmd: InfoCommand) -> anyhow::Result<()> {
  Ok(())
}
