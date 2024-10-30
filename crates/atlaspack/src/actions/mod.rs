pub mod asset_graph;
pub mod entry;
pub mod target;

use asset_graph::AssetGraphAction;
use entry::EntryAction;
use target::TargetAction;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum ActionType {
  AssetGraph(AssetGraphAction),
  Entry(EntryAction),
  Target(TargetAction),
}

impl std::fmt::Display for ActionType {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    match self {
      Self::AssetGraph(_) => write!(f, "AssetGraph"),
      Self::Entry(_) => write!(f, "Entry"),
      Self::Target(_) => write!(f, "Target"),
    }
  }
}

#[derive(Clone)]
pub struct ActionQueue(UnboundedSender<ActionType>);

impl From<UnboundedSender<ActionType>> for ActionQueue {
  fn from(tx: UnboundedSender<ActionType>) -> Self {
    Self(tx)
  }
}

impl ActionQueue {
  pub fn new() -> (Self, UnboundedReceiver<ActionType>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (tx.into(), rx)
  }

  pub fn next(
    &self,
    a: ActionType,
  ) -> anyhow::Result<()> {
    Ok(self.0.send(a)?)
  }
}
