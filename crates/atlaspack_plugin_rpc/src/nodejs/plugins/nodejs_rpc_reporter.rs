use std::fmt;
use std::fmt::Debug;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::ReporterPlugin;

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

impl ReporterPlugin for NodejsRpcReporterPlugin {
  fn report(&self, _event: &ReporterEvent) -> Result<(), anyhow::Error> {
    // TODO
    Ok(())
  }
}
