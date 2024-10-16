use std::fmt;
use std::fmt::Debug;

use atlaspack_config::PluginNode;
use atlaspack_core::bundle_graph::BundleGraph;
use atlaspack_core::plugin::BundlerPlugin;
use atlaspack_core::plugin::PluginContext;

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

impl BundlerPlugin for NodejsRpcBundlerPlugin {
  fn bundle(&self, _bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error> {
    todo!()
  }

  fn optimize(&self, _bundle_graph: &mut BundleGraph) -> Result<(), anyhow::Error> {
    todo!()
  }
}
