mod plugins;
mod resolver;
mod transformer;

use dyn_hash::DynHash;
use std::sync::Arc;

use atlaspack_core::plugin::Resolved;

pub use plugins::*;
pub use resolver::*;
pub use transformer::*;

pub type RpcHostRef = Arc<dyn RpcHost>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

pub trait RpcHost: Send + Sync + DynHash {
  fn start(&self) -> anyhow::Result<RpcWorkerRef>;
}

dyn_hash::hash_trait_object!(RpcHost);

pub trait RpcWorker: Send + Sync + DynHash {
  fn ping(&self) -> anyhow::Result<()>;
  fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()>;
  fn run_resolver_resolve(&self, opts: RunResolverResolve) -> anyhow::Result<Resolved>;
  fn run_transformer(&self, opts: RpcTransformerOpts) -> anyhow::Result<RpcTransformerResult>;
}

dyn_hash::hash_trait_object!(RpcWorker);
