mod plugins;
mod resolver;
mod transformer;

use std::sync::Arc;

pub use plugins::*;
pub use resolver::*;
pub use transformer::*;

pub type RpcHostRef = Arc<dyn RpcHost>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

pub trait RpcHost: Send + Sync {
  fn start(&self) -> anyhow::Result<RpcWorkerRef>;
}

pub trait RpcWorker: Send + Sync {
  fn ping(&self) -> anyhow::Result<()>;
  fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()>;
  fn run_transformer(&self, opts: RpcTransformerOpts) -> anyhow::Result<RpcTransformerResult>;
}
