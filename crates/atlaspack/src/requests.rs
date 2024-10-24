use std::fmt::Display;

pub use asset_graph_request::*;
use asset_request::AssetRequestOutput;
use entry_request::EntryRequestOutput;
use path_request::PathRequestOutput;
use target_request::TargetRequestOutput;

mod asset_graph_request;
mod asset_request;
mod entry_request;
mod path_request;
mod target_request;

/// Union of all request outputs
#[derive(Clone, Debug, PartialEq)]
pub enum RequestResult {
  Done,
  AssetGraph(AssetGraphRequestOutput),
  Asset(AssetRequestOutput),
  Entry(EntryRequestOutput),
  Path(PathRequestOutput),
  Target(TargetRequestOutput),
  // The following are test request types only used in the test build
  #[cfg(test)]
  TestSub(String),
  #[cfg(test)]
  TestMain(Vec<String>),
}

impl Display for RequestResult {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RequestResult::Done => write!(f, "Done"),
      RequestResult::AssetGraph(_) => write!(f, "AssetGraph"),
      RequestResult::Asset(_) => write!(f, "Asset"),
      RequestResult::Entry(_) => write!(f, "Entry"),
      RequestResult::Path(_) => write!(f, "Path"),
      RequestResult::Target(_) => write!(f, "Target"),
      #[cfg(test)]
      RequestResult::TestSub(_) => write!(f, "TestSub"),
      #[cfg(test)]
      RequestResult::TestMain(_) => write!(f, "TestMain"),
    }
  }
}
