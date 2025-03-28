use std::sync::Arc;

use once_cell::sync::OnceCell;

use super::super::super::RpcFactory;
use super::super::super::RpcWorker;
use super::nodejs_rpc_worker::NodejsWorker;
use super::nodejs_rpc_worker_farm::NodejsWorkerFarm;

/// NodejsRpcFactory represents the main JavaScript thread and
/// facilitates the spawning of Nodejs worker threads
pub struct NodejsRpcFactory {
  workers: OnceCell<Arc<NodejsWorkerFarm>>,
  rx_workers: Vec<Arc<NodejsWorker>>,
}

impl NodejsRpcFactory {
  pub fn new(rx_workers: Vec<Arc<NodejsWorker>>) -> napi::Result<Self> {
    Ok(Self {
      workers: Default::default(),
      rx_workers,
    })
  }
}

impl RpcFactory for NodejsRpcFactory {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>> {
    // Get the workers the first time `start` is called and reuse
    // them each time
    Ok(
      self
        .workers
        .get_or_try_init({
          let rx_workers = self.rx_workers.clone();
          move || -> anyhow::Result<Arc<NodejsWorkerFarm>> {
            Ok(Arc::new(NodejsWorkerFarm::new(rx_workers.clone())))
          }
        })?
        .clone(),
    )
  }
}
