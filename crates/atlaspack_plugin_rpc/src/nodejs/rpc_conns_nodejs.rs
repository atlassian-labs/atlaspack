use std::sync::atomic::{AtomicUsize, Ordering};

use atlaspack_core::plugin::Resolved;

use super::NodejsWorker;

use crate::{LoadPluginOptions, RpcTransformerOpts, RpcTransformerResult, RpcWorker};

/// Connection to multiple Nodejs Workers
/// Implements round robin messaging
pub struct NodejsWorkerFarm {
  current_index: AtomicUsize,
  workers: Vec<NodejsWorker>,
}

impl std::hash::Hash for NodejsWorkerFarm {
  fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl NodejsWorkerFarm {
  pub fn new(workers: Vec<NodejsWorker>) -> Self {
    Self {
      current_index: Default::default(),
      workers,
    }
  }

  fn next_index(&self) -> usize {
    self
      .current_index
      .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some((value + 1) % self.workers.len())
      })
      .expect("Could not get worker")
  }

  fn next_worker(&self) -> &NodejsWorker {
    &self.workers[self.next_index()]
  }
}

impl RpcWorker for NodejsWorkerFarm {
  fn ping(&self) -> anyhow::Result<()> {
    for worker in &self.workers {
      worker.ping()?;
    }
    Ok(())
  }

  fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    for worker in &self.workers {
      worker.load_plugin(opts.clone())?;
    }
    Ok(())
  }

  fn run_resolver_resolve(&self, opts: crate::RunResolverResolve) -> anyhow::Result<Resolved> {
    self.next_worker().run_resolver_resolve(opts)
  }

  fn run_transformer(&self, opts: RpcTransformerOpts) -> anyhow::Result<RpcTransformerResult> {
    self.next_worker().run_transformer(opts)
  }
}
