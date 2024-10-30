use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::types::Environment;

use super::super::ActionQueue;
use super::super::ActionType;
use super::super::Compilation;
use super::super::TargetAction;
use crate::actions::Action;

#[derive(Debug, Hash)]
pub struct AssetAction {
  pub project_root: PathBuf,
  pub code: Option<String>,
  pub env: Arc<Environment>,
  pub file_path: PathBuf,
  pub pipeline: Option<String>,
  pub query: Option<String>,
  pub side_effects: bool,
}

impl Action for AssetAction {
  async fn run(
    self,
    q: ActionQueue,
    Compilation { .. }: &Compilation,
  ) -> anyhow::Result<()> {
    Ok(())
  }
}
