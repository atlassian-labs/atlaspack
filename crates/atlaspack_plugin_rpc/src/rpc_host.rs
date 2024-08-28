use std::path::Path;
use std::sync::Arc;

pub type RpcHostRef = Arc<dyn RpcHost>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

pub trait RpcHost: Send + Sync {
  fn start(&self) -> anyhow::Result<RpcWorkerRef>;
}

pub trait RpcWorker: Send + Sync {
  fn ping(&self) -> anyhow::Result<()>;
  fn register_transformer(&self, resolve_from: &Path, specifier: &str) -> anyhow::Result<()>;
}
