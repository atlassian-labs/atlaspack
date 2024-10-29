use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::*;

use super::super::super::RpcWorker;
use super::super::plugins::*;
use super::nodejs_rpc_worker::NodejsWorker;
use super::LoadPluginOptions;

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

  pub fn next_worker(&self) -> Arc<NodejsWorker> {
    self.workers[self.next_index()].clone()
  }

  pub fn all_workers(&self) -> Vec<Arc<NodejsWorker>> {
    let mut workers = vec![];

    for worker in self.workers.iter() {
      workers.push(worker.clone());
    }

    workers
  }

  pub async fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    let mut set = vec![];

    for worker in self.all_workers() {
      let opts = opts.clone();
      set.push(tokio::spawn(async move { worker.load_plugin(opts).await }));
    }

    while let Some(res) = set.pop() {
      res.await??;
    }

    Ok(())
  }
}
