use std::future::Future;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::*;

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
    Self {
      workers: Arc::new(NodeJsWorkerCollection {
        current_index: Default::default(),
        workers,
      }),
    }
  }
}

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

  fn create_transformer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>> {
    Ok(Arc::new(NodejsRpcTransformerPlugin::new(
      self.workers.clone(),
      ctx,
      plugin,
    )?))
  }

  fn create_bundler(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn BundlerPlugin>> {
    Ok(Box::new(NodejsRpcBundlerPlugin::new(ctx, plugin)?))
  }

  fn create_compressor(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn CompressorPlugin>> {
    Ok(Box::new(NodejsRpcCompressorPlugin::new(ctx, plugin)?))
  }

  fn create_namer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn NamerPlugin>> {
    Ok(Box::new(NodejsRpcNamerPlugin::new(ctx, plugin)?))
  }

  fn create_optimizer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn OptimizerPlugin>> {
    Ok(Box::new(NodejsRpcOptimizerPlugin::new(ctx, plugin)?))
  }

  fn create_packager(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn PackagerPlugin>> {
    Ok(Box::new(NodejsRpcPackagerPlugin::new(ctx, plugin)?))
  }

  fn create_reporter(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn ReporterPlugin>> {
    Ok(Box::new(NodejsRpcReporterPlugin::new(ctx, plugin)?))
  }

  fn create_runtime(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn RuntimePlugin>> {
    Ok(Box::new(NodejsRpcRuntimePlugin::new(ctx, plugin)?))
  }
}

pub struct NodeJsWorkerCollection {
  current_index: AtomicUsize,
  workers: Vec<Arc<NodejsWorker>>,
}

impl NodeJsWorkerCollection {
  fn next_index(&self) -> usize {
    self
      .current_index
      .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some((value + 1) % self.workers.len())
      })
      .expect("Could not get worker")
  }

  fn next_worker(&self) -> Arc<NodejsWorker> {
    self.workers[self.next_index()].clone()
  }

  /// Execute function on one Nodejs worker thread
  pub async fn exec_on_one<R, F, Fut>(&self, eval: F) -> anyhow::Result<R>
  where
    F: FnOnce(Arc<NodejsWorker>) -> Fut,
    Fut: Future<Output = anyhow::Result<R>>,
  {
    eval(self.next_worker()).await
  }

  /// Execute function on all Nodejs worker threads
  pub async fn exec_on_all<R, F, Fut>(&self, eval: F) -> anyhow::Result<Vec<R>>
  where
    F: Fn(Arc<NodejsWorker>) -> Fut,
    Fut: Future<Output = anyhow::Result<R>>,
  {
    let mut results = vec![];

    for worker in self.workers.iter() {
      results.push(eval(worker.clone()).await?)
    }

    Ok(results)
  }
}
