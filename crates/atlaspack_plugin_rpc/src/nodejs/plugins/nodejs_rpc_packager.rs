use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PackageContext;
use atlaspack_core::plugin::PackagedBundle;
use atlaspack_core::plugin::PackagerPlugin;
use atlaspack_core::plugin::PluginContext;
use std::fmt;
use std::fmt::Debug;

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

#[async_trait]
impl PackagerPlugin for NodejsRpcPackagerPlugin {
  async fn package<'a>(&self, _ctx: PackageContext<'a>) -> Result<PackagedBundle, anyhow::Error> {
    todo!()
  }
}
