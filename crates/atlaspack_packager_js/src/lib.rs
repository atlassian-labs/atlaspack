use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::cache::CacheRef;
use atlaspack_core::debug_tools::DebugTools;
use lmdb_js_lite::DatabaseHandle;
use parking_lot::RwLock;

/// Context object containing all the dependencies needed for packaging bundles.
/// This groups related configuration and avoids passing many individual parameters.
pub struct PackagingContext {
  pub db: Arc<DatabaseHandle>,
  pub cache: CacheRef,
  pub project_root: PathBuf,
  pub debug_tools: DebugTools,
  /// When true, top-level `require()` variable declarations are removed and their
  /// usages are replaced with inline `(0, require("id"))` expressions. This defers
  /// module initialisation to first use, improving startup performance.
  pub inline_requires: bool,
}

pub struct JsPackager<B: BundleGraph + Send + Sync> {
  context: PackagingContext,
  bundle_graph: Arc<RwLock<B>>,
}

pub mod inline_requires;
pub mod js_packager;
pub mod process_asset;
