use std::fmt::Debug;
use std::path::PathBuf;

use crate::bundle_graph::BundleGraph;
use crate::types::Bundle;
use crate::types::Dependency;
use async_trait::async_trait;

pub enum RuntimeAssetPriority {
  Sync,
  Parallel,
}

/// A "synthetic" asset that will be inserted into the bundle graph
pub struct RuntimeAsset {
  pub code: String,
  pub dependency: Option<Dependency>,
  pub file_path: PathBuf,
  pub is_entry: Option<bool>,
  pub priority: Option<RuntimeAssetPriority>,
  // TODO env: Option<EnvironmentOptions>
}

/// Programmatically insert assets into bundles
#[async_trait]
pub trait RuntimePlugin: Debug + Send + Sync {
  /// Generates runtime assets to insert into bundles
  async fn apply(
    &self,
    bundle: Bundle,
    bundle_graph: BundleGraph,
  ) -> Result<Option<Vec<RuntimeAsset>>, anyhow::Error>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestRuntimePlugin {}

  #[async_trait]
  impl RuntimePlugin for TestRuntimePlugin {
    async fn apply(
      &self,
      _bundle: Bundle,
      _bundle_graph: BundleGraph,
    ) -> Result<Option<Vec<RuntimeAsset>>, anyhow::Error> {
      todo!()
    }
  }

  #[test]
  fn can_be_defined_in_dyn_vec() {
    let runtimes: Vec<Box<dyn RuntimePlugin>> = vec![Box::new(TestRuntimePlugin {})];

    assert_eq!(runtimes.len(), 1);
  }
}
