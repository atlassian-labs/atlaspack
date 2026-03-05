pub use asset_graph_request::*;
use asset_request::AssetRequestOutput;
pub use build_request::*;
pub use bundle_graph_request::*;
use entry_request::EntryRequestOutput;
use package_request::PackageRequestOutput;
use path_request::PathRequestOutput;
use target_request::TargetRequestOutput;

mod asset_graph_request;
mod asset_request;
mod build_request;
mod bundle_graph_request;
mod entry_request;
mod package_request;
mod path_request;
mod target_request;

/// Union of all request outputs
#[derive(Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum RequestResult {
  AssetGraph(AssetGraphRequestOutput),
  Asset(AssetRequestOutput),
  Build(BuildRequestOutput),
  BundleGraph(BundleGraphRequestOutput),
  Entry(EntryRequestOutput),
  Path(PathRequestOutput),
  Target(TargetRequestOutput),
  Package(PackageRequestOutput),
  // The following are test request types only used in the test build
  #[cfg(test)]
  TestSub(String),
  #[cfg(test)]
  TestMain(Vec<String>),
}

impl std::fmt::Display for RequestResult {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      RequestResult::AssetGraph(_output) => f.write_str("AssetGraph"),
      RequestResult::Build(_output) => f.write_str("Build"),
      RequestResult::BundleGraph(_output) => f.write_str("BundleGraph"),
      RequestResult::Entry(output) => f.write_str(&format!("Entry({:?})", &output.entries)),
      RequestResult::Asset(output) => {
        f.write_str(&format!("Asset({})", &output.asset.file_path.display()))
      }
      RequestResult::Path(output) => f.write_str(&format!("Path({:?})", output)),
      RequestResult::Target(_output) => f.write_str(&format!("Target")),
      RequestResult::Package(_output) => f.write_str(&format!("Package")),
      #[cfg(test)]
      RequestResult::TestSub(_output) => f.write_str(&format!("TestSub")),
      #[cfg(test)]
      RequestResult::TestMain(_output) => f.write_str(&format!("Target")),
    }
  }
}

impl std::fmt::Debug for RequestResult {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      RequestResult::AssetGraph(output) => f.debug_tuple("AssetGraph").field(output).finish(),
      RequestResult::Asset(output) => f.debug_tuple("Asset").field(output).finish(),
      RequestResult::Build(output) => f.debug_tuple("Build").field(output).finish(),
      RequestResult::BundleGraph(output) => f.debug_tuple("BundleGraph").field(output).finish(),
      RequestResult::Entry(output) => f.debug_tuple("Entry").field(output).finish(),
      RequestResult::Path(output) => f.debug_tuple("Path").field(output).finish(),
      RequestResult::Target(output) => f.debug_tuple("Target").field(output).finish(),
      RequestResult::Package(output) => f.debug_tuple("Package").field(output).finish(),
      #[cfg(test)]
      RequestResult::TestSub(output) => f.debug_tuple("TestSub").field(output).finish(),
      #[cfg(test)]
      RequestResult::TestMain(output) => f.debug_tuple("TestMain").field(output).finish(),
    }
  }
}
