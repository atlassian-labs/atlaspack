use std::path::Path;
use std::sync::Arc;

use atlaspack_core::plugin::TransformResult;
use atlaspack_core::types::Asset;

pub type RpcHostRef = Arc<dyn RpcHost>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

pub trait RpcHost: Send + Sync {
  fn start(&self) -> anyhow::Result<RpcWorkerRef>;
}

pub trait RpcWorker: Send + Sync {
  fn ping(&self) -> anyhow::Result<()>;
  fn register_transformer(&self, resolve_from: &Path, specifier: &str) -> anyhow::Result<()>;
  fn transform_transformer(&self, key: &str, asset: &Asset) -> anyhow::Result<TransformResult>;
}
