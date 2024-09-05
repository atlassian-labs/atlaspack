use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Error;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Environment;

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
      // TODO: Fix or remove the duplicate meta fields. I vote we remove these fields as they're just duplicates of meta
      ..Default::default()
    };

    let transformed_dependencies = result
      .dependencies
      .into_iter()
      .map(|dep| {
        // Copy the Asset environmnet or merge the new environment with the Asset environment
        let env: Arc<Environment> = dep.env.map_or_else(
          || asset_env.clone(),
          |dep_env| {
            Arc::new(Environment {
              context: dep_env.context.unwrap_or_else(|| asset_env.context),
              engines: dep_env.engines.unwrap_or_else(|| asset_env.engines.clone()),
              include_node_modules: dep_env
                .include_node_modules
                .unwrap_or_else(|| asset_env.include_node_modules.clone()),
              is_library: dep_env.is_library.unwrap_or_else(|| asset_env.is_library),
              loc: dep_env.loc,
              output_format: dep_env
                .output_format
                .unwrap_or_else(|| asset_env.output_format),
              should_scope_hoist: dep_env
                .should_scope_hoist
                .unwrap_or_else(|| asset_env.should_scope_hoist),
              should_optimize: dep_env
                .should_optimize
                .unwrap_or_else(|| asset_env.should_optimize),
              // TODO: Handle source maps
              source_map: None,
              source_type: dep_env.source_type.unwrap_or_else(|| asset_env.source_type),
            })
          },
        );

        Dependency {
          id: dep.id,
          bundle_behavior: dep.bundle_behavior.unwrap_or_default(),
          env,
          loc: dep.loc,
          meta: dep.meta,
          package_conditions: dep.package_conditions.unwrap_or_default(),
          pipeline: dep.pipeline,
          priority: dep.priority.unwrap_or_default(),
          range: dep.range,
          resolve_from: dep.resolve_from,
          source_asset_id: Some(transformed_asset.id.clone()),
          source_path: Some(transformed_asset.file_path.clone()),
          specifier: dep.specifier,
          specifier_type: dep.specifier_type,
          symbols: dep.symbols,
          target: None,
          // TODO: Fix or remove the duplicate meta fields. I vote we remove these fields as they're just duplicates of meta
          ..Default::default()
        }
      })
      .collect();

    Ok(TransformResult {
      asset: transformed_asset,
      dependencies: transformed_dependencies,
      // TODO: Handle invalidations
      invalidate_on_file_change: Vec::new(),
    })
  }
}
