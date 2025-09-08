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
      RequestResult::AssetGraph(_) => write!(f, "AssetGraph"),
      RequestResult::Asset(asset_request) => {
        write!(f, "Asset({:?})", asset_request.asset.file_path)
      }
      RequestResult::Entry(_) => write!(f, "Entry"),
      RequestResult::Path(_) => write!(f, "Path"),
      RequestResult::Target(output) => output.fmt(f),
      #[cfg(test)]
      RequestResult::TestSub(_) => write!(f, "TestSub"),
      #[cfg(test)]
      RequestResult::TestMain(_) => write!(f, "TestMain"),
    }
  }
}
