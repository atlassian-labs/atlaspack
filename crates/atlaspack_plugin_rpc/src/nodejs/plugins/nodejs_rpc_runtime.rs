use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::bundle_graph::BundleGraph;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::RuntimeAsset;
use atlaspack_core::plugin::RuntimePlugin;
use atlaspack_core::types::Bundle;
use std::fmt;
use std::fmt::Debug;

pub struct NodejsRpcRuntimePlugin {
  _name: String,
}

impl Debug for NodejsRpcRuntimePlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcRuntimePlugin")
  }
}

impl NodejsRpcRuntimePlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> Result<Self, anyhow::Error> {
    Ok(NodejsRpcRuntimePlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

#[async_trait]
impl RuntimePlugin for NodejsRpcRuntimePlugin {
  async fn apply(
    &self,
    _bundle: Bundle,
    _bundle_graph: BundleGraph,
  ) -> Result<Option<Vec<RuntimeAsset>>, anyhow::Error> {
    todo!()
  }
}
