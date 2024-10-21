use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::plugin::PluginOptions;
use once_cell::sync::OnceCell;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Error;
use atlaspack_core::hash::IdentifierHasher;
use atlaspack_napi_helpers::anyhow_from_napi;
use serde::Deserialize;
use serde::Serialize;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::Asset;
use atlaspack_core::types::*;

use super::super::rpc::nodejs_rpc_worker_farm::NodeJsWorkerCollection;
use super::super::rpc::LoadPluginKind;
use super::super::rpc::LoadPluginOptions;
use super::plugin_options::RpcPluginOptions;

pub struct NodejsRpcTransformerPlugin {
  nodejs_workers: Arc<NodeJsWorkerCollection>,
  config_loader: ConfigLoaderRef,
  plugin_options: Arc<PluginOptions>,
  plugin_node: PluginNode,
  started: OnceCell<Vec<()>>,
}

impl Debug for NodejsRpcTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcTransformerPlugin")
  }
}

impl NodejsRpcTransformerPlugin {
  pub fn new(
    nodejs_workers: Arc<NodeJsWorkerCollection>,
    ctx: &PluginContext,
    plugin_node: &PluginNode,
  ) -> Result<Self, anyhow::Error> {
    Ok(Self {
      nodejs_workers,
      config_loader: ctx.config.clone(),
      plugin_options: ctx.options.clone(),
      plugin_node: plugin_node.clone(),
      started: OnceCell::new(),
    })
  }

  fn bootstrap(&self) -> anyhow::Result<RpcPluginOptions> {
    self.started.get_or_try_init(|| {
      self.nodejs_workers.exec_on_all(|worker| {
        worker.load_plugin(
          LoadPluginOptions {
            kind: LoadPluginKind::Transformer,
            specifier: self.plugin_node.package_name.clone(),
            resolve_from: (&*self.plugin_node.resolve_from).clone(),
          },
          self.config_loader.clone(),
        )
      })
    })?;

    Ok(RpcPluginOptions {
      hmr_options: None,
      project_root: self.plugin_options.project_root.clone(),
      mode: self.plugin_options.mode.clone(),
    })
  }
}

impl TransformerPlugin for NodejsRpcTransformerPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.plugin_node.hash(&mut hasher);
    hasher.finish()
  }

  fn transform(
    &mut self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let rpc_plugin_options = self.bootstrap()?;
    let asset_env = asset.env.clone();
    let stats = asset.stats.clone();

    let run_transformer_opts = RpcTransformerOpts {
      key: self.plugin_node.package_name.clone(),
      options: rpc_plugin_options.clone(),
      env: asset_env.clone(),
      asset,
    };

    let result: RpcTransformerResult = self.nodejs_workers.exec_on_one(|worker| {
      worker
        .transformer_register_fn
        .call_with_return_serde(run_transformer_opts)
        .map_err(anyhow_from_napi)
    })?;

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

/// This Asset mostly replicates the core Asset type however it only features
/// fields that can be modified by transformers
#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAssetResult {
  pub id: String,
  pub bundle_behavior: Option<BundleBehavior>,
  pub file_path: PathBuf,
  #[serde(rename = "type")]
  pub file_type: FileType,
  pub code: Code,
  pub meta: JSONObject,
  pub pipeline: Option<String>,
  pub query: Option<String>,
  pub symbols: Option<Vec<Symbol>>,
  pub unique_key: Option<String>,
  pub side_effects: bool,
  pub is_bundle_splittable: bool,
  pub is_source: bool,
}

// This Environment mostly replicates the core Environment but makes everything
// optional as it is merged into the Asset Environment later.
// Similar to packages/core/types-internal/src/index.js#EnvironmnetOptions
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcEnvironmentResult {
  pub context: Option<EnvironmentContext>,
  pub engines: Option<Engines>,
  pub include_node_modules: Option<IncludeNodeModules>,
  pub is_library: Option<bool>,
  pub loc: Option<SourceLocation>,
  pub output_format: Option<OutputFormat>,
  pub should_scope_hoist: Option<bool>,
  pub should_optimize: Option<bool>,
  pub source_type: Option<SourceType>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcTransformerOpts {
  pub key: String,
  pub options: RpcPluginOptions,
  pub env: Arc<Environment>,
  pub asset: Asset,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcTransformerResult {
  pub asset: RpcAssetResult,
}
