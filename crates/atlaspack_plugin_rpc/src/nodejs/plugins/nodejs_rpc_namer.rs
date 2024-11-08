use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::bundle_graph::BundleGraph;
use atlaspack_core::plugin::NamerPlugin;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::types::Bundle;
use std::fmt;
use std::fmt::Debug;

pub struct NodejsRpcNamerPlugin {
  _name: String,
}

impl Debug for NodejsRpcNamerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcNamerPlugin")
  }
}

impl NodejsRpcNamerPlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> Result<Self, anyhow::Error> {
    Ok(NodejsRpcNamerPlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

#[async_trait]
impl NamerPlugin for NodejsRpcNamerPlugin {
  async fn name(
    &self,
    _bundle: &Bundle,
    _bundle_graph: &BundleGraph,
  ) -> Result<Option<std::path::PathBuf>, anyhow::Error> {
    todo!()
  }
}
