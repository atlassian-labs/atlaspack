use std::sync::Arc;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::*;
use atlaspack_core::types::Asset;

use crate::RpcFactory;
use crate::RpcWorker;

#[derive(Default)]
pub struct TestingRpcFactory {}

impl RpcFactory for TestingRpcFactory {
  fn start(&self) -> anyhow::Result<std::sync::Arc<dyn RpcWorker>> {
    Ok(Arc::new(TestingRpcWorker {}))
  }
}

#[derive(Default)]
pub struct TestingRpcWorker {}

impl RpcWorker for TestingRpcWorker {
  fn create_bundler(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn BundlerPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcBundlerPlugin".into())))
  }

  fn create_compressor(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn CompressorPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcCompressorPlugin".into())))
  }

  fn create_namer(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn NamerPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcNamerPlugin".into())))
  }

  fn create_optimizer(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn OptimizerPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcOptimizerPlugin".into())))
  }

  fn create_packager(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn PackagerPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcPackagerPlugin".into())))
  }

  fn create_reporter(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn ReporterPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcReporterPlugin".into())))
  }

  fn create_resolver(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn ResolverPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcResolverPlugin".into())))
  }

  fn create_runtime(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn RuntimePlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcRuntimePlugin".into())))
  }

  fn create_transformer(
    &self,
    _ctx: &PluginContext,
    _plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn TransformerPlugin>> {
    Ok(Box::new(TestingRpcPlugin("RpcTransformerPlugin".into())))
  }
}

pub struct TestingRpcPlugin(String);

impl std::fmt::Debug for TestingRpcPlugin {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl BundlerPlugin for TestingRpcPlugin {
  fn bundle(
    &self,
    _bundle_graph: &mut atlaspack_core::bundle_graph::BundleGraph,
  ) -> Result<(), anyhow::Error> {
    Ok(())
  }

  fn optimize(
    &self,
    _bundle_graph: &mut atlaspack_core::bundle_graph::BundleGraph,
  ) -> Result<(), anyhow::Error> {
    Ok(())
  }
}

impl CompressorPlugin for TestingRpcPlugin {
  fn compress(&self, _file: &std::fs::File) -> Result<Option<CompressedFile>, String> {
    Ok(None)
  }
}

impl NamerPlugin for TestingRpcPlugin {
  fn name(
    &self,
    _bundle: &atlaspack_core::types::Bundle,
    _bundle_graph: &atlaspack_core::bundle_graph::BundleGraph,
  ) -> Result<Option<std::path::PathBuf>, anyhow::Error> {
    Ok(None)
  }
}

impl OptimizerPlugin for TestingRpcPlugin {
  fn optimize(&self, _ctx: OptimizeContext) -> Result<OptimizedBundle, anyhow::Error> {
    anyhow::bail!("Mock Optimizer Plugin Incomplete")
    // Ok(OptimizedBundle {
    //   contents: fs::File::create(),
    // })
  }
}

impl PackagerPlugin for TestingRpcPlugin {
  fn package(&self, _ctx: PackageContext) -> Result<PackagedBundle, anyhow::Error> {
    anyhow::bail!("Mock Packager Plugin Incomplete")
    // Ok(PackagedBundle {
    //   contents: fs::File::create(),
    // })
  }
}

impl ReporterPlugin for TestingRpcPlugin {
  fn report(&self, _event: &ReporterEvent) -> Result<(), anyhow::Error> {
    Ok(())
  }
}

impl ResolverPlugin for TestingRpcPlugin {
  fn resolve(&self, _ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
    Ok(Resolved {
      invalidations: vec![],
      resolution: Resolution::Unresolved,
    })
  }
}

impl RuntimePlugin for TestingRpcPlugin {
  fn apply(
    &self,
    _bundle: atlaspack_core::types::Bundle,
    _bundle_graph: atlaspack_core::bundle_graph::BundleGraph,
  ) -> Result<Option<Vec<RuntimeAsset>>, anyhow::Error> {
    Ok(None)
  }
}

impl TransformerPlugin for TestingRpcPlugin {
  fn transform(
    &mut self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, anyhow::Error> {
    Ok(TransformResult {
      asset,
      dependencies: vec![],
      discovered_assets: vec![],
      invalidate_on_file_change: vec![],
    })
  }
}
