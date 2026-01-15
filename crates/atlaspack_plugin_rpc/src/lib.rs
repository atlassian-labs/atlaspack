#[cfg(feature = "nodejs")]
pub mod nodejs;
pub mod testing;

use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::*;
use atlaspack_package_manager::PackageManagerRef;
use mockall::automock;

pub type RpcFactoryRef = Arc<dyn RpcFactory>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

/// RpcFactory is a connection to an external execution context.
/// An example of an external execution context is Nodejs via Napi
///
/// The RpcFactory is used to spawn an RpcWorker which is used to
/// execute the tasks like the offloading work to plugins in the
/// external execution context.
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
#[automock]
pub trait RpcFactory: Send + Sync {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>>;
}

/// RpcWorker is a connection to a specific worker within the
/// external context.
///
/// This is used to map calls to the bundler plugins APIs to
/// their equivalent within the external context
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
#[automock]
#[async_trait]
pub trait RpcWorker: Send + Sync {
  fn create_resolver(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn ResolverPlugin>>;
  async fn create_transformer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
    package_manager: PackageManagerRef,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>>;
}
