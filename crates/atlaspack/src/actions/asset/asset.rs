use std::path::PathBuf;
use std::sync::Arc;

use super::super::ActionQueue;
use super::super::ActionType;
use super::super::TargetAction;
use crate::compilation::Compilation;

#[derive(Debug)]
pub struct AssetAction {
  pub entry: String,
}

impl AssetAction {
  pub async fn run(
    self,
    q: ActionQueue,
    Compilation { .. }: &Compilation,
  ) -> anyhow::Result<()> {
    Ok(())
  }
}
