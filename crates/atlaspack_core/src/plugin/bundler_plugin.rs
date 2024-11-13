use crate::bundle_graph::BundleGraph;
use async_trait::async_trait;
use std::fmt::Debug;

/// Converts an asset graph into a BundleGraph
///
/// Bundlers accept the entire asset graph and modify it to add bundle nodes that group the assets
/// into output bundles.
///
/// Bundle and optimize run in series and are functionally identitical.
///
#[async_trait]
pub trait BundlerPlugin: Debug + Send + Sync {
  // TODO: Should BundleGraph be AssetGraph or something that contains AssetGraph in the name?
  async fn bundle(&self, bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error>;

  async fn optimize(&self, bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestBundlerPlugin {}

  #[async_trait]
  impl BundlerPlugin for TestBundlerPlugin {
    async fn bundle(&self, _bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error> {
      todo!()
    }

    async fn optimize(&self, _bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error> {
      todo!()
    }
  }

  #[test]
  fn can_be_dyn() {
    let _bundler: Box<dyn BundlerPlugin> = Box::new(TestBundlerPlugin {});
  }
}
