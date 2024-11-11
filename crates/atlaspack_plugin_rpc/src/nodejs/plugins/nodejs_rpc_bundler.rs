use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::bundle_graph::BundleGraph;
use atlaspack_core::plugin::BundlerPlugin;
use atlaspack_core::plugin::PluginContext;
use std::fmt;
use std::fmt::Debug;

pub struct NodejsRpcBundlerPlugin {
  _name: String,
}

impl Debug for NodejsRpcBundlerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcBundlerPlugin")
  }
}

impl NodejsRpcBundlerPlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> Result<Self, anyhow::Error> {
    Ok(NodejsRpcBundlerPlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

#[async_trait]
impl BundlerPlugin for NodejsRpcBundlerPlugin {
  async fn bundle(&self, _bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error> {
    todo!()
  }

  async fn optimize(&self, _bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error> {
    todo!()
  }
}
