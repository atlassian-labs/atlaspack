pub mod testing {
  use std::borrow::Cow;
  use std::sync::Arc;

  use async_trait::async_trait;
  use atlaspack_config::PluginNode;
  use atlaspack_core::plugin::*;
  use atlaspack_core::types::Asset;
  use atlaspack_package_manager::PackageManagerRef;

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

  #[async_trait]
  impl RpcWorker for TestingRpcWorker {
    fn create_resolver(
      &self,
      _ctx: &PluginContext,
      _plugin: &PluginNode,
    ) -> anyhow::Result<Arc<dyn ResolverPlugin>> {
      Ok(Arc::new(TestingRpcPlugin("RpcResolverPlugin".into())))
    }

    async fn create_transformer(
      &self,
      _ctx: &PluginContext,
      _plugin: &PluginNode,
      _package_manager: PackageManagerRef,
    ) -> anyhow::Result<Arc<dyn TransformerPlugin>> {
      Ok(Arc::new(TestingRpcPlugin("RpcTransformerPlugin".into())))
    }
  }

  pub struct TestingRpcPlugin(String);

  impl std::fmt::Debug for TestingRpcPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  #[async_trait]
  impl ResolverPlugin for TestingRpcPlugin {
    async fn resolve(&self, _ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
      Ok(Resolved {
        invalidations: vec![],
        resolution: Resolution::Unresolved,
      })
    }
  }
  impl CacheKey for TestingRpcPlugin {
    fn cache_key(&self) -> Cow<'_, CacheStatus> {
      Cow::Owned(CacheStatus::Uncachable)
    }
  }

  #[async_trait]
  impl TransformerPlugin for TestingRpcPlugin {
    async fn transform(&self, asset: Asset) -> Result<TransformResult, anyhow::Error> {
      Ok(TransformResult {
        asset,
        dependencies: vec![],
        discovered_assets: vec![],
        invalidate_on_file_change: vec![],
        cache_bailout: false,
      })
    }
  }
}
