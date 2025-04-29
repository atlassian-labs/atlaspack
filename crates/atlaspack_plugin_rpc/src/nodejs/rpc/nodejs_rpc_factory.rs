use std::sync::Arc;

use super::super::super::RpcFactory;
use super::super::super::RpcWorker;
use super::nodejs_rpc_worker::NodejsWorker;
use super::nodejs_rpc_worker_farm::NodejsWorkerFarm;

/// NodejsRpcFactory represents the main JavaScript thread and
/// facilitates the spawning of Nodejs worker threads
pub struct NodejsRpcFactory {
  worker_farm: Arc<NodejsWorkerFarm>,
}

impl NodejsRpcFactory {
  pub fn new(workers: Vec<Arc<NodejsWorker>>) -> napi::Result<Self> {
    Ok(Self {
      worker_farm: Arc::new(NodejsWorkerFarm::new(workers)),
    })
  }
}

impl RpcFactory for NodejsRpcFactory {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>> {
    Ok(self.worker_farm.clone())
  }
}
