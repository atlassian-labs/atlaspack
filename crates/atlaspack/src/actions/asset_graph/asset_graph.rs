use std::sync::Arc;

use super::super::entry::EntryAction;
use super::super::ActionQueue;
use super::super::ActionType;
use crate::compilation::Compilation;

#[derive(Debug)]
pub struct AssetGraphAction {}

impl AssetGraphAction {
  pub fn new() -> Self {
    Self {}
  }

  pub async fn run(
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
