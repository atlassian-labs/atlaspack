use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::debug_tools::DebugTools;
use lmdb_js_lite::DatabaseHandle;
use parking_lot::RwLock;

pub struct JsPackager<B: BundleGraph + Send + Sync> {
  db: Arc<DatabaseHandle>,
  bundle_graph: Arc<RwLock<B>>,
  project_root: PathBuf,
  debug_tools: DebugTools,
}

pub mod js_packager;
pub mod process_asset;
