use super::install::InstallCommand;
use crate::context::Context;

/// Reinstall a specified version of Atlaspack.
/// Useful for re-installing git branches
pub fn main(ctx: Context, cmd: InstallCommand) -> anyhow::Result<()> {
  super::install::main(ctx, InstallCommand { force: true, ..cmd })
}
