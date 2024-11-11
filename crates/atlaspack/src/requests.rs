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

impl RequestResult {
  pub fn as_entry(&self) -> Option<&EntryRequestOutput> {
    match self {
      RequestResult::Entry(output) => Some(output),
      _ => None,
    }
  }

  pub fn as_asset(&self) -> Option<&AssetRequestOutput> {
    match self {
      RequestResult::Asset(output) => Some(output),
      _ => None,
    }
  }

  pub fn as_path(&self) -> Option<&PathRequestOutput> {
    match self {
      RequestResult::Path(output) => Some(output),
      _ => None,
    }
  }

  pub fn as_target(&self) -> Option<&TargetRequestOutput> {
    match self {
      RequestResult::Target(output) => Some(output),
      _ => None,
    }
  }

  pub fn as_asset_graph(&self) -> Option<&AssetGraphRequestOutput> {
    match self {
      RequestResult::AssetGraph(output) => Some(output),
      _ => None,
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use asset_graph_request::AssetGraphRequestOutput;
  use asset_request::AssetRequestOutput;
  use entry_request::EntryRequestOutput;
  use path_request::PathRequestOutput;
  use target_request::TargetRequestOutput;

  #[test]
  fn test_as_entry() {
    let entry_output = EntryRequestOutput { entries: vec![] };
    let result = RequestResult::Entry(entry_output.clone());
    assert_eq!(result.as_entry(), Some(&entry_output));
  }

  #[test]
  fn test_as_asset() {
    let asset_output = AssetRequestOutput {
      asset: Default::default(),
      discovered_assets: vec![],
      dependencies: vec![],
    };
    let result = RequestResult::Asset(asset_output.clone());
    assert_eq!(result.as_asset(), Some(&asset_output));
  }

  #[test]
  fn test_as_path() {
    let path_output = PathRequestOutput::Excluded;
    let result = RequestResult::Path(path_output.clone());
    assert_eq!(result.as_path(), Some(&path_output));
  }

  #[test]
  fn test_as_target() {
    let target_output = TargetRequestOutput {
      entry: Default::default(),
      targets: vec![],
    };
    let result = RequestResult::Target(target_output.clone());
    assert_eq!(result.as_target(), Some(&target_output));
  }

  #[test]
  fn test_as_asset_graph() {
    let asset_graph_output = AssetGraphRequestOutput {
      graph: Default::default(),
    };
    let result = RequestResult::AssetGraph(asset_graph_output.clone());
    assert_eq!(result.as_asset_graph(), Some(&asset_graph_output));
  }
}
