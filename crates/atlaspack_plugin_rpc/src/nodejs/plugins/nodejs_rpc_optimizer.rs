use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::OptimizeContext;
use atlaspack_core::plugin::OptimizedBundle;
use atlaspack_core::plugin::OptimizerPlugin;
use atlaspack_core::plugin::PluginContext;
use std::fmt;
use std::fmt::Debug;

pub struct NodejsRpcOptimizerPlugin {
  _name: String,
}

impl Debug for NodejsRpcOptimizerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcOptimizerPlugin")
  }
}

impl NodejsRpcOptimizerPlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> Result<Self, anyhow::Error> {
    Ok(NodejsRpcOptimizerPlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

#[async_trait]
impl OptimizerPlugin for NodejsRpcOptimizerPlugin {
  async fn optimize<'a>(
    &self,
    _ctx: OptimizeContext<'a>,
  ) -> Result<OptimizedBundle, anyhow::Error> {
    todo!()
  }
}
