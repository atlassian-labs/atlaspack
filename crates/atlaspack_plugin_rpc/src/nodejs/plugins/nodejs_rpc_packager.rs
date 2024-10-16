use std::fmt;
use std::fmt::Debug;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PackageContext;
use atlaspack_core::plugin::PackagedBundle;
use atlaspack_core::plugin::PackagerPlugin;
use atlaspack_core::plugin::PluginContext;

pub struct NodejsRpcPackagerPlugin {
  _name: String,
}

impl Debug for NodejsRpcPackagerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcPackagerPlugin")
  }
}

impl NodejsRpcPackagerPlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> Result<Self, anyhow::Error> {
    Ok(NodejsRpcPackagerPlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

impl PackagerPlugin for NodejsRpcPackagerPlugin {
  fn package(&self, _ctx: PackageContext) -> Result<PackagedBundle, anyhow::Error> {
    todo!()
  }
}
