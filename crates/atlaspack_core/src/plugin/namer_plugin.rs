use crate::bundle_graph::BundleGraph;
use crate::types::Bundle;
use async_trait::async_trait;
use std::fmt::Debug;
use std::path::PathBuf;

/// Determines the output filename for a bundle
///
/// Namers run in a pipeline until one returns a result.
///
#[async_trait]
pub trait NamerPlugin: Debug + Send + Sync {
  /// Names the given bundle
  ///
  /// The returned file path should be relative to the target dist directory, and will be used to
  /// name the bundle. Naming can be forwarded onto the next plugin by returning None.
  ///
  async fn name(
    &self,
    bundle: &Bundle,
    bundle_graph: &BundleGraph,
  ) -> Result<Option<PathBuf>, anyhow::Error>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestNamerPlugin {}

  #[async_trait]
  impl NamerPlugin for TestNamerPlugin {
    async fn name(
      &self,
      _bundle: &Bundle,
      _bundle_graph: &BundleGraph,
    ) -> Result<Option<PathBuf>, anyhow::Error> {
      todo!()
    }
  }

  #[test]
  fn can_be_defined_in_dyn_vec() {
    let mut namers = Vec::<Box<dyn NamerPlugin>>::new();

    namers.push(Box::new(TestNamerPlugin {}));

    assert_eq!(namers.len(), 1);
  }
}
