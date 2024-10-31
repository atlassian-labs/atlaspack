pub mod asset;
pub mod asset_graph;
mod compilation;
pub mod entry;
pub mod path;
pub mod target;

use std::hash::Hash;
use std::hash::Hasher;

use asset::AssetAction;
use asset_graph::AssetGraphAction;
use entry::EntryAction;
use path::PathAction;
use target::TargetAction;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

pub use self::compilation::Compilation;

#[derive(Hash, Debug)]
pub enum ActionType {
  AssetGraph(AssetGraphAction),
  Entry(EntryAction),
  Target(TargetAction),
  Path(PathAction),
  Asset(AssetAction),
}

impl Action for ActionType {
  async fn run(
    self,
    q: ActionQueue,
    c: &Compilation,
  ) -> anyhow::Result<()> {
    match self {
      ActionType::Entry(a) => a.run(q, &c).await,
      ActionType::AssetGraph(a) => a.run(q, &c).await,
      ActionType::Target(a) => a.run(q, &c).await,
      ActionType::Path(a) => a.run(q, &c).await,
      ActionType::Asset(a) => a.run(q, &c).await,
    }
  }
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
      Self::Path(_) => write!(f, "Path"),
      Self::Asset(_) => write!(f, "Asset"),
    }
  }
}

pub trait Action: Hash + Send + Sync {
  fn id(&self) -> u64 {
    let mut hasher = atlaspack_core::hash::IdentifierHasher::default();
    std::any::type_name::<Self>().hash(&mut hasher);
    self.hash(&mut hasher);
    hasher.finish()
  }

  async fn run(
    self,
    q: ActionQueue,
    c: &Compilation,
  ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct ActionQueue(UnboundedSender<(ActionType, u64)>);

impl ActionQueue {
  pub fn new() -> (Self, UnboundedReceiver<(ActionType, u64)>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (Self(tx), rx)
  }

  pub fn next(
    &self,
    a: ActionType,
  ) -> anyhow::Result<()> {
    let id = a.id();
    Ok(self.0.send((a, id))?)
  }
}
