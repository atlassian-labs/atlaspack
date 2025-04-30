use super::install::InstallCommand;
use crate::context::Context;

pub fn main(ctx: Context, cmd: InstallCommand) -> anyhow::Result<()> {
  super::install::main(ctx, InstallCommand { force: true, ..cmd })
}
