use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Error;

use atlaspack_config::PluginNode;
use atlaspack_core::hash::IdentifierHasher;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;

use crate::RpcPluginOptions;
use crate::RpcTransformerOpts;
use crate::RpcWorkerRef;

pub struct RpcTransformerPlugin {
  rpc_worker: RpcWorkerRef,
  plugin: PluginNode,
  plugin_options: RpcPluginOptions,
}

impl Debug for RpcTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcTransformerPlugin({})", self.plugin.package_name)
  }
}

impl RpcTransformerPlugin {
  pub fn new(
    rpc_worker: RpcWorkerRef,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> Result<Self, anyhow::Error> {
    let plugin_options = RpcPluginOptions {
      hmr_options: None,
      project_root: ctx.options.project_root.clone(),
    };
    Ok(RpcTransformerPlugin {
      rpc_worker,
      plugin: plugin.clone(),
      plugin_options,
    })
  }
}

impl TransformerPlugin for RpcTransformerPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.plugin.hash(&mut hasher);
    hasher.finish()
  }

  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    let asset_env = asset.env.clone();
    let stats = asset.stats.clone();
    let run_transformer_opts = RpcTransformerOpts {
      // TODO: Make this not bad
      resolve_from: self.plugin.resolve_from.to_str().unwrap().to_string(),
      specifier: self.plugin.package_name.clone(),
      // TODO: Pass this just once to each worker
      options: self.plugin_options.clone(),
      asset,
    };

    let result = self.rpc_worker.run_transformer(run_transformer_opts)?;

    let transformed_asset = Asset {
      id: result.asset.id,
      code: Arc::new(result.asset.code),
      bundle_behavior: result.asset.bundle_behavior,
      env: asset_env.clone(),
      file_path: result.asset.file_path,
      file_type: result.asset.file_type,
      meta: result.asset.meta,
      pipeline: result.asset.pipeline,
      query: result.asset.query,
      stats,
      symbols: result.asset.symbols,
      unique_key: result.asset.unique_key,
      side_effects: result.asset.side_effects,
      is_bundle_splittable: result.asset.is_bundle_splittable,
      is_source: result.asset.is_source,
      // TODO: Fix or remove the duplicate meta fields.
      ..Default::default()
    };

    Ok(TransformResult {
      // Adding dependencies from Node plugins isn't yet supported
      // TODO: Handle invalidations
      asset: transformed_asset,
      ..Default::default()
    })
  }
}
