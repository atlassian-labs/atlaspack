use std::path::Path;

use parking_lot::Mutex;

use super::NodejsWorker;

use crate::RpcWorker;

/// Connection to multiple Nodejs Workers
/// Implements round robin messaging
pub struct NodejsWorkerFarm {
  current_index: Mutex<usize>, // TODO use AtomicUsize
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
  fn next_index(&self) -> usize {
    let mut current_index = self.current_index.lock();
    if *current_index >= self.workers.len() - 1 {
      *current_index = 0;
    } else {
      *current_index = *current_index + 1;
    }
    current_index.clone()
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
}
