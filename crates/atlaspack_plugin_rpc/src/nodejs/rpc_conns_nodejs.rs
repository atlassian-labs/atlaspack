use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use super::NodejsWorker;

use crate::RpcWorker;

/// Connection to multiple Nodejs Workers
/// Implements round robin messaging
pub struct NodejsWorkerFarm {
  current_index: AtomicUsize,
  workers: Vec<NodejsWorker>,
}

impl NodejsWorkerFarm {
  pub fn new(workers: Vec<NodejsWorker>) -> Self {
    Self {
      current_index: Default::default(),
      workers,
    }
  }

  #[allow(unused)]
  fn next_index(&self) -> anyhow::Result<usize> {
    self
      .current_index
      .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some((value + 1) % self.workers.len())
      })
      .map_err(|v| anyhow::anyhow!(v))
  }
}

impl RpcWorker for NodejsWorkerFarm {
  fn ping(&self) -> anyhow::Result<()> {
    for worker in &self.workers {
      worker.ping()?;
    }
    Ok(())
  }

  fn register_transformer(&self, resolve_from: &Path, specifier: &str) -> anyhow::Result<()> {
    for worker in &self.workers {
      worker.register_transformer(resolve_from, specifier)?;
    }
    Ok(())
  }

  fn transform_transformer(
    &self,
    key: &str,
    asset: &atlaspack_core::types::Asset,
  ) -> anyhow::Result<atlaspack_core::plugin::TransformResult> {
    self.workers[self.next_index()?].transform_transformer(key, asset)
  }
}
