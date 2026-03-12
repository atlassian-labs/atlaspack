use std::sync::Arc;

use anyhow::Result;
use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;

use crate::{CssPackager, CssPackagingContext};

impl<B: BundleGraph + Send + Sync> CssPackager<B> {
  pub fn new(context: CssPackagingContext, bundle_graph: Arc<B>) -> Self {
    Self {
      context,
      bundle_graph,
    }
  }

  /// Packages the CSS bundle.
  ///
  /// Full implementation is tracked separately.
  pub fn package(&self, _bundle_id: &str) -> Result<()> {
    todo!("CSS packaging not yet implemented")
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn css_packaging_context_fields_are_accessible() {
    let context = CssPackagingContext {
      project_root: PathBuf::from("/tmp/project"),
      output_dir: PathBuf::from("/tmp/project/dist"),
    };
    assert_eq!(context.project_root, PathBuf::from("/tmp/project"));
    assert_eq!(context.output_dir, PathBuf::from("/tmp/project/dist"));
  }
}
