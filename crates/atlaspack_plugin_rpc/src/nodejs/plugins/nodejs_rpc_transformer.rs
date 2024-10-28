use async_trait::async_trait;
use atlaspack_core::plugin::PluginOptions;
use atlaspack_napi_helpers::ZeroCopyBuffer;
use napi::bindgen_prelude::FromNapiValue;
use napi::JsObject;
use napi::JsUnknown;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

use anyhow::Error;
use atlaspack_core::hash::IdentifierHasher;
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

/// Plugin state once initialized
struct InitializedState {
  rpc_plugin_options: RpcPluginOptions,
}

pub struct NodejsRpcTransformerPlugin {
  nodejs_workers: Arc<NodeJsWorkerCollection>,
  plugin_options: Arc<PluginOptions>,
  plugin_node: PluginNode,
  started: OnceCell<InitializedState>,
}

impl Debug for NodejsRpcTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcTransformerPlugin({})", self.plugin_node.package_name)
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
      plugin_options: ctx.options.clone(),
      plugin_node: plugin_node.clone(),
      started: OnceCell::new(),
    })
  }

  async fn get_or_init_state(&self) -> anyhow::Result<&InitializedState> {
    self
      .started
      .get_or_try_init::<anyhow::Error, _, _>(|| async move {
        self
          .nodejs_workers
          .load_plugin(LoadPluginOptions {
            kind: LoadPluginKind::Transformer,
            specifier: self.plugin_node.package_name.clone(),
            #[allow(clippy::needless_borrow)]
            resolve_from: (&*self.plugin_node.resolve_from).clone(),
          })
          .await?;

        Ok(InitializedState {
          rpc_plugin_options: RpcPluginOptions {
            hmr_options: None,
            project_root: self.plugin_options.project_root.clone(),
            mode: self.plugin_options.mode.clone(),
          },
        })
      })
      .await
  }
}

#[async_trait]
impl TransformerPlugin for NodejsRpcTransformerPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.plugin_node.hash(&mut hasher);
    hasher.finish()
  }

  async fn transform(
    &self,
    _context: TransformContext,
    mut asset: Asset,
  ) -> Result<TransformResult, Error> {
    let state = self.get_or_init_state().await?;

    let asset_env = asset.env.clone();
    let stats = asset.stats.clone();

    let run_transformer_opts = RpcTransformerOpts {
      key: self.plugin_node.package_name.clone(),
      options: state.rpc_plugin_options.clone(),
      env: asset_env.clone(),
      asset: Asset {
        code: Default::default(),
        ..asset
      },
    };

    let (result, contents) = self
      .nodejs_workers
      .next_worker()
      .transformer_register_fn
      .call(
        move |env| {
          let run_transformer_opts = env.to_js_value(&run_transformer_opts)?;
          let bytes = std::mem::take(asset.code.get_mut());
          let buf = ZeroCopyBuffer::new(&env, bytes)?;
          Ok(vec![run_transformer_opts, buf.into_unknown()])
        },
        |env, return_value| {
          let return_value = JsObject::from_unknown(return_value)?;

          let transform_result = return_value.get_element_unchecked::<JsUnknown>(0)?;
          let transform_result = env.from_js_value::<RpcTransformerResult, _>(transform_result)?;

          let contents = return_value.get_element_unchecked::<ZeroCopyBuffer>(1)?;
          let contents = contents.to_vec()?;

          Ok((transform_result, contents))
        },
      )
      .await?;

    let transformed_asset = Asset {
      id: result.asset.id,
      code: Code::new(contents),
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
