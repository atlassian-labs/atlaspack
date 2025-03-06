use std::sync::mpsc::Receiver;
use std::sync::Arc;

use parking_lot::Mutex;

use super::super::super::RpcFactory;
use super::super::super::RpcWorker;
use super::nodejs_rpc_worker::NodejsWorker;
use super::nodejs_rpc_worker_farm::NodejsWorkerFarm;

/// NodejsRpcFactory represents the main JavaScript thread and
/// facilitates the spawning of Nodejs worker threads
pub struct NodejsRpcFactory {
  rx_workers: Mutex<Receiver<Vec<Arc<NodejsWorker>>>>,
}

impl NodejsRpcFactory {
  pub fn new(rx_workers: Receiver<Vec<Arc<NodejsWorker>>>) -> napi::Result<Self> {
    Ok(Self {
      rx_workers: Mutex::new(rx_workers),
    })
  }
}

impl RpcFactory for NodejsRpcFactory {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>> {
    let rx_workers = self.rx_workers.lock();
    Ok(Arc::new(NodejsWorkerFarm::new(rx_workers.recv().unwrap())))
  }
}
