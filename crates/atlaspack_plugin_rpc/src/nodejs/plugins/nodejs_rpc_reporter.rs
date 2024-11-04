use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::ReporterPlugin;
use std::fmt;
use std::fmt::Debug;

pub struct NodejsRpcReporterPlugin {
  _name: String,
}

impl Debug for NodejsRpcReporterPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcReporterPlugin")
  }
}

impl NodejsRpcReporterPlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> anyhow::Result<Self> {
    Ok(NodejsRpcReporterPlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

#[async_trait]
impl ReporterPlugin for NodejsRpcReporterPlugin {
  async fn report(&self, _event: &ReporterEvent) -> Result<(), anyhow::Error> {
    // TODO
    Ok(())
  }
}
