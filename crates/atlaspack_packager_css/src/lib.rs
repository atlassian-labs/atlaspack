use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;

pub mod css_packager;

/// Context provided to the CSS packager for each packaging run.
///
/// Mirrors the essential fields from `PackagingContext` in `atlaspack_packager_js`. Both
/// packagers need the same resources; consolidating into a shared type in `atlaspack_core`
/// is tracked as a follow-up (see AFB-1911 open questions). For now this is defined here
/// to avoid a cross-packager-crate dependency.
///
/// TODO(AFB-1911): Consolidate with `PackagingContext` in `atlaspack_packager_js` into a
/// shared type in `atlaspack_core`.
pub struct CssPackagingContext {
  /// Absolute path to the project root directory.
  pub project_root: PathBuf,
  /// Output directory where bundle files are written.
  pub output_dir: PathBuf,
}

/// Native Rust CSS packager.
///
/// Packages a CSS bundle into its final output form. Full implementation is tracked in
/// AFB-1912 (core bundling), AFB-1913 (URL replacement), AFB-1915 (source maps), and
/// AFB-1916 (CSS Modules tree-shaking).
pub struct CssPackager<B: BundleGraph + Send + Sync> {
  #[allow(dead_code)]
  context: CssPackagingContext,
  #[allow(dead_code)]
  bundle_graph: Arc<B>,
}
