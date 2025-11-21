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
#[derive(Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum RequestResult {
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

impl std::fmt::Debug for RequestResult {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      RequestResult::AssetGraph(output) => f.debug_tuple("AssetGraph").field(output).finish(),
      RequestResult::Asset(output) => f.debug_tuple("Asset").field(output).finish(),
      RequestResult::Entry(output) => f.debug_tuple("Entry").field(output).finish(),
      RequestResult::Path(output) => f.debug_tuple("Path").field(output).finish(),
      RequestResult::Target(output) => f.debug_tuple("Target").field(output).finish(),
      #[cfg(test)]
      RequestResult::TestSub(output) => f.debug_tuple("TestSub").field(output).finish(),
      #[cfg(test)]
      RequestResult::TestMain(output) => f.debug_tuple("TestMain").field(output).finish(),
    }
  }
}
