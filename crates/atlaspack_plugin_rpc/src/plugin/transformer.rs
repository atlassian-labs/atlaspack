use std::fmt;
use std::fmt::Debug;

use anyhow::Error;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;

use crate::RpcWorkerRef;

pub struct RpcTransformerPlugin {
  _rpc_worker: RpcWorkerRef,
  name: String,
}

impl Debug for RpcTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcTransformerPlugin({})", self.name)
  }
}

impl RpcTransformerPlugin {
  pub fn new(
    rpc_worker: RpcWorkerRef,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> Result<Self, anyhow::Error> {
    rpc_worker.register_transformer(&plugin.resolve_from, &plugin.package_name)?;

    Ok(RpcTransformerPlugin {
      _rpc_worker: rpc_worker,
      name: plugin.package_name.clone(),
    })
  }
}

impl TransformerPlugin for RpcTransformerPlugin {
  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    Ok(TransformResult {
      asset,
      dependencies: vec![],
      invalidate_on_file_change: vec![],
    })
  }
}
