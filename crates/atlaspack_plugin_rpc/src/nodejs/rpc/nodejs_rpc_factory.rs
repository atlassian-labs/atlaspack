use std::sync::mpsc::Receiver;
use std::sync::Arc;

use anyhow::anyhow;
use parking_lot::Mutex;

use super::super::super::RpcFactory;
use super::super::super::RpcWorker;
use super::nodejs_rpc_worker::NodejsWorker;
use super::nodejs_rpc_worker_farm::NodejsWorkerFarm;

/// NodejsRpcFactory represents the main JavaScript thread and
/// facilitates the spawning of Nodejs worker threads
pub struct NodejsRpcFactory {
  node_workers: usize,
  rx_worker: Mutex<Receiver<NodejsWorker>>,
}

impl NodejsRpcFactory {
  pub fn new(node_workers: usize, rx_worker: Receiver<NodejsWorker>) -> napi::Result<Self> {
    Ok(Self {
      node_workers,
      rx_worker: Mutex::new(rx_worker),
    })
  }
}

impl RpcFactory for NodejsRpcFactory {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>> {
    let rx_worker = self.rx_worker.lock();
    let mut workers = vec![];

    for _ in 0..self.node_workers {
      let Ok(worker) = rx_worker.recv() else {
        return Err(anyhow!("Unable to receive NodejsWorker"));
      };
      workers.push(Arc::new(worker))
    }

    Ok(Arc::new(NodejsWorkerFarm::new(workers)))
  }
}
