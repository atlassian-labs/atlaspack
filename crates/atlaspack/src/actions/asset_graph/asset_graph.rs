use std::sync::Arc;

use super::super::entry::EntryAction;
use super::super::ActionQueue;
use super::super::ActionType;
use super::super::Compilation;
use crate::actions::Action;

#[derive(Hash, Debug)]
pub struct AssetGraphAction {}

impl Action for AssetGraphAction {
  async fn run(
    self,
    q: ActionQueue,
    Compilation { entries, .. }: &Compilation,
  ) -> anyhow::Result<()> {
    for entry in entries.iter() {
      q.next(ActionType::Entry(EntryAction {
        entry: entry.clone(),
      }))?;
    }
    Ok(())
  }
}
