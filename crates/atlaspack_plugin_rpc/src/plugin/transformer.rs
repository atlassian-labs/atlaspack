use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Error;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;
use once_cell::sync::OnceCell;

use crate::RpcWorkerRef;

pub struct RpcTransformerPlugin {
  key: Arc<OnceCell<String>>,
  rpc_worker: RpcWorkerRef,
  plugin: PluginNode,
}

impl Debug for RpcTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcTransformerPlugin({})", self.plugin.package_name)
  }
}

impl RpcTransformerPlugin {
  pub fn new(
    rpc_worker: RpcWorkerRef,
    _ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> Result<Self, anyhow::Error> {
    Ok(RpcTransformerPlugin {
      key: Default::default(),
      rpc_worker,
      plugin: plugin.clone(),
    })
  }

  fn get_key(&self) -> anyhow::Result<&String> {
    self.key.get_or_try_init(move || {
      (&self.rpc_worker)
        .register_transformer(&self.plugin.resolve_from, &self.plugin.package_name)?;
      Ok(format!(
        "{}:{}",
        &self.plugin.resolve_from.to_str().unwrap(),
        &self.plugin.package_name
      ))
    })
  }
}

impl TransformerPlugin for RpcTransformerPlugin {
  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    let key = self.get_key()?;

    println!("{:?} {}", self, key);
    self.rpc_worker.transform_transformer(key, &asset)?;

    Ok(TransformResult {
      asset,
      dependencies: vec![],
      invalidate_on_file_change: vec![],
    })
  }
}
