#[cfg(feature = "nodejs")]
pub mod nodejs;
pub mod testing;

use std::sync::Arc;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::*;

pub type RpcFactoryRef = Arc<dyn RpcFactory>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

/// RpcFactory is a connection to an external execution context.
/// An example of an external execution context is Nodejs via Napi
///
/// The RpcFactory is used to spawn an RpcWorker which is used to
/// execute the tasks like the offloading work to plugins in the
/// external execution context.
pub trait RpcFactory: Send + Sync {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>>;
}

/// RpcWorker is a connection to a specific worker within the
/// external context.
///
/// This is used to map calls to the bundler plugins APIs to
/// their equivalent within the external context
pub trait RpcWorker: Send + Sync {
  fn create_bundler(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn BundlerPlugin>>;
  fn create_compressor(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn CompressorPlugin>>;
  fn create_namer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn NamerPlugin>>;
  fn create_optimizer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn OptimizerPlugin>>;
  fn create_packager(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn PackagerPlugin>>;
  fn create_reporter(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn ReporterPlugin>>;
  fn create_resolver(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn ResolverPlugin>>;
  fn create_runtime(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn RuntimePlugin>>;
  fn create_transformer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>>;
}
