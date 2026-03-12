use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;

pub mod css_packager;

/// Context provided to the CSS packager.
///
/// Mirrors `PackagingContext` in `atlaspack_packager_js`. Consolidating into a shared
/// type in `atlaspack_core` is planned.
pub struct CssPackagingContext {
  /// Absolute path to the project root directory.
  pub project_root: PathBuf,
  /// Output directory where bundle files are written.
  pub output_dir: PathBuf,
}

/// Native Rust CSS packager.
///
/// Full implementation is tracked separately.
pub struct CssPackager<B: BundleGraph + Send + Sync> {
  #[allow(dead_code)]
  context: CssPackagingContext,
  #[allow(dead_code)]
  bundle_graph: Arc<B>,
}
