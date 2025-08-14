use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::CompressorPlugin;
use atlaspack_core::plugin::NamerPlugin;
use atlaspack_core::plugin::OptimizerPlugin;
use atlaspack_core::plugin::PackagerPlugin;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ReporterPlugin;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::RuntimePlugin;
use atlaspack_core::plugin::TransformerPlugin;

use crate::RpcFactory;
use crate::RpcWorker;

#[derive(Default)]
pub struct RustWorkerFactory {}

impl RpcFactory for RustWorkerFactory {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>> {
    Ok(Arc::new(RustWorker::default()))
  }
}

#[derive(Default)]
pub struct RustWorker {}

impl RpcWorker for RustWorker {
  fn create_bundler(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn atlaspack_core::plugin::BundlerPlugin>> {
    tracing::debug!(?plugin, "create_bundler");
    Err(anyhow::anyhow!("create_bundler: Not implemented"))
  }

  fn create_compressor(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn CompressorPlugin>> {
    tracing::debug!(?plugin, "create_compressor");
    Err(anyhow::anyhow!("create_compressor: Not implemented"))
  }

  fn create_namer(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn NamerPlugin>> {
    tracing::debug!(?plugin, "create_namer");
    Err(anyhow::anyhow!("create_namer: Not implemented"))
  }

  fn create_optimizer(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn OptimizerPlugin>> {
    tracing::debug!(?plugin, "create_optimizer");
    Err(anyhow::anyhow!("create_optimizer: Not implemented"))
  }

  fn create_packager(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn PackagerPlugin>> {
    tracing::debug!(?plugin, "create_packager");
    Err(anyhow::anyhow!("create_packager: Not implemented"))
  }

  fn create_reporter(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn ReporterPlugin>> {
    tracing::debug!(?plugin, "create_reporter");

    #[derive(Debug)]
    struct NoopReporterPlugin {}

    #[async_trait]
    impl ReporterPlugin for NoopReporterPlugin {
      async fn report(
        &self,
        _event: &atlaspack_core::plugin::ReporterEvent,
      ) -> Result<(), anyhow::Error> {
        Ok(())
      }
    }

    Ok(Box::new(NoopReporterPlugin {}))
  }

  fn create_resolver(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn ResolverPlugin>> {
    tracing::debug!(?plugin, "create_resolver");
    Err(anyhow::anyhow!("create_resolver: Not implemented"))
  }

  fn create_runtime(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn RuntimePlugin>> {
    tracing::debug!(?plugin, "create_runtime");
    Err(anyhow::anyhow!("create_runtime: Not implemented"))
  }

  fn create_transformer(
    &self,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>> {
    tracing::debug!(?plugin, "create_transformer");
    Err(anyhow::anyhow!("create_transformer: Not implemented"))
  }
}
