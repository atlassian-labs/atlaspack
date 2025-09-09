pub use asset_graph_request::*;
use asset_request::AssetRequestOutput;
use atlaspack_core::{as_variant_impl, into_variant_impl};
use bundle_graph_request::BundleGraphRequestOutput;
use entry_request::EntryRequestOutput;
use package_request::PackageRequestOutput;
use path_request::PathRequestOutput;
use target_request::TargetRequestOutput;

pub mod asset_graph_request;
pub mod asset_request;
pub mod bundle_graph_request;
pub mod entry_request;
pub mod package_request;
pub mod path_request;
pub mod target_request;

/// Union of all request outputs
#[derive(Clone, PartialEq)]
pub enum RequestResult {
  AssetGraph(AssetGraphRequestOutput),
  Asset(AssetRequestOutput),
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

into_variant_impl!(
  RequestResult,
  into_asset_graph,
  AssetGraph,
  AssetGraphRequestOutput
);
into_variant_impl!(RequestResult, into_asset, Asset, AssetRequestOutput);
into_variant_impl!(
  RequestResult,
  into_bundle_graph,
  BundleGraph,
  BundleGraphRequestOutput
);
into_variant_impl!(RequestResult, into_entry, Entry, EntryRequestOutput);
into_variant_impl!(RequestResult, into_path, Path, PathRequestOutput);
into_variant_impl!(RequestResult, into_target, Target, TargetRequestOutput);
into_variant_impl!(RequestResult, into_package, Package, PackageRequestOutput);
as_variant_impl!(
  RequestResult,
  as_asset_graph,
  AssetGraph,
  AssetGraphRequestOutput
);
as_variant_impl!(RequestResult, as_asset, Asset, AssetRequestOutput);
as_variant_impl!(
  RequestResult,
  as_bundle_graph,
  BundleGraph,
  BundleGraphRequestOutput
);
as_variant_impl!(RequestResult, as_entry, Entry, EntryRequestOutput);
as_variant_impl!(RequestResult, as_path, Path, PathRequestOutput);
as_variant_impl!(RequestResult, as_target, Target, TargetRequestOutput);
as_variant_impl!(RequestResult, as_package, Package, PackageRequestOutput);

impl std::fmt::Debug for RequestResult {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      RequestResult::AssetGraph(_) => write!(f, "AssetGraph"),
      RequestResult::Asset(asset_request) => {
        write!(f, "Asset({:?})", asset_request.asset.file_path)
      }
      RequestResult::BundleGraph(_) => write!(f, "BundleGraph"),
      RequestResult::Entry(_) => write!(f, "Entry"),
      RequestResult::Path(_) => write!(f, "Path"),
      RequestResult::Target(output) => output.fmt(f),
      #[cfg(test)]
      RequestResult::TestSub(_) => write!(f, "TestSub"),
      #[cfg(test)]
      RequestResult::TestMain(_) => write!(f, "TestMain"),
      RequestResult::Package(_) => write!(f, "Package"),
    }
  }
}
