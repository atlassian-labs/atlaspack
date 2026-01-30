use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::*;
use atlaspack_package_manager::PackageManagerRef;

use super::super::super::RpcWorker;
use super::super::plugins::*;
use super::nodejs_rpc_worker::NodejsWorker;

/// NodejsWorkerFarm holds a collection of Nodejs worker threads
/// and provides the ability to initialize plugins
pub struct NodejsWorkerFarm {
  workers: Arc<NodeJsWorkerCollection>,
}

impl NodejsWorkerFarm {
  pub fn new(workers: Vec<Arc<NodejsWorker>>) -> Self {
    let workers_with_busyness = workers
      .into_iter()
      .map(|worker| {
        Arc::new(WorkerWithBusyness {
          worker,
          active_tasks: AtomicUsize::new(0),
        })
      })
      .collect();

    Self {
      workers: Arc::new(NodeJsWorkerCollection {
        workers: workers_with_busyness,
      }),
    }
  }
}

#[async_trait]
impl RpcWorker for NodejsWorkerFarm {
  fn create_resolver(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> Result<Arc<dyn ResolverPlugin>, anyhow::Error> {
    Ok(Arc::new(RpcNodejsResolverPlugin::new(
      self.workers.clone(),
      ctx,
      plugin,
    )?))
  }

  async fn create_transformer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
    package_manager: PackageManagerRef,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>> {
    Ok(Arc::new(
      NodejsRpcTransformerPlugin::new(self.workers.clone(), ctx, plugin, package_manager).await?,
    ))
  }
}

pub struct NodeJsWorkerCollection {
  workers: Vec<Arc<WorkerWithBusyness>>,
}

struct WorkerWithBusyness {
  worker: Arc<NodejsWorker>,
  active_tasks: AtomicUsize,
}

/// A guard that tracks a worker's busyness and automatically decrements
/// the active task count when dropped
pub struct BusyWorkerGuard {
  worker: Arc<NodejsWorker>,
  busy_worker: Arc<WorkerWithBusyness>,
}

impl std::ops::Deref for BusyWorkerGuard {
  type Target = NodejsWorker;

  fn deref(&self) -> &Self::Target {
    &self.worker
  }
}

impl Drop for BusyWorkerGuard {
  fn drop(&mut self) {
    // Decrement the active task count when the guard is dropped
    self
      .busy_worker
      .active_tasks
      .fetch_sub(1, Ordering::Relaxed);
  }
}

impl NodeJsWorkerCollection {
  pub fn next_worker(&self) -> BusyWorkerGuard {
    // Find the worker with the least number of active tasks
    let mut least_busy_index = 0;
    let mut min_tasks = self.workers[0].active_tasks.load(Ordering::Relaxed);

    for (index, worker) in self.workers.iter().enumerate().skip(1) {
      let tasks = worker.active_tasks.load(Ordering::Relaxed);
      if tasks < min_tasks {
        min_tasks = tasks;
        least_busy_index = index;
      }
    }

    let selected_worker = &self.workers[least_busy_index];

    // Increment the task count for the selected worker
    selected_worker.active_tasks.fetch_add(1, Ordering::Relaxed);

    BusyWorkerGuard {
      worker: selected_worker.worker.clone(),
      busy_worker: selected_worker.clone(),
    }
  }

  pub fn all_workers(&self) -> Vec<Arc<NodejsWorker>> {
    let mut workers = vec![];

    for worker_with_busyness in self.workers.iter() {
      workers.push(worker_with_busyness.worker.clone());
    }

    workers
  }
}
