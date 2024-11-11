use crate::bundle_graph::BundleGraph;
use crate::types::Bundle;
use crate::types::SourceMap;
use async_trait::async_trait;
use std::fmt::Debug;
use std::fs::File;

pub struct PackageContext<'a> {
  pub bundle: &'a Bundle,
  pub bundle_graph: &'a BundleGraph,
  pub contents: &'a File, // TODO We may want this to be a String or File later
  pub map: Option<&'a SourceMap>,
  // TODO getSourceMapReference?
}

pub struct PackagedBundle {
  pub contents: File,
  // TODO ast, map, type
}

/// Combines all the assets in a bundle together into an output file
///
/// Packagers are also responsible for resolving URL references, bundle inlining, and generating
/// source maps.
///
#[async_trait]
pub trait PackagerPlugin: Debug + Send + Sync {
  /// Combines assets in a bundle
  async fn package<'a>(&self, ctx: PackageContext<'a>) -> Result<PackagedBundle, anyhow::Error>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestPackagerPlugin {}

  #[async_trait]
  impl PackagerPlugin for TestPackagerPlugin {
    async fn package<'a>(&self, _ctx: PackageContext<'a>) -> Result<PackagedBundle, anyhow::Error> {
      todo!()
    }
  }

  #[test]
  fn can_be_dyn() {
    let _packager: Box<dyn PackagerPlugin> = Box::new(TestPackagerPlugin {});
  }
}
