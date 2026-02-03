use std::sync::Arc;

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use lmdb_js_lite::DatabaseHandle;
use parking_lot::RwLock;

pub struct JsPackager<B: BundleGraph + Send + Sync> {
  db: Arc<DatabaseHandle>,
  bundle_graph: Arc<RwLock<B>>,
}

pub mod js_packager;
