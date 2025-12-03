use anyhow::Context;
use async_trait::async_trait;
use atlaspack_core::plugin::CacheStatus;
use atlaspack_core::plugin::PluginOptions;
use atlaspack_package_manager::PackageManagerRef;
use napi::JsBuffer;
use napi::JsObject;
use napi::JsString;
use napi::JsUnknown;
use napi::bindgen_prelude::FromNapiValue;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task;

use anyhow::Error;
use atlaspack_core::hash::IdentifierHasher;
use serde::Deserialize;
use serde::Serialize;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::*;

use crate::nodejs::conditions::Conditions;
use crate::nodejs::conditions::SerializableTransformerConditions;
use crate::nodejs::plugins::load_plugin::LoadPluginKind;
use crate::nodejs::plugins::load_plugin::LoadPluginOptions;

use super::super::rpc::nodejs_rpc_worker_farm::NodeJsWorkerCollection;
use super::load_plugin::RpcPluginOptions;

pub mod conditions;

pub struct NodejsRpcTransformerPlugin {
  nodejs_workers: Arc<NodeJsWorkerCollection>,
  plugin_options: Arc<PluginOptions>,
  plugin_node: PluginNode,
  conditions: Conditions,
  cache_key: CacheStatus,
}

impl Debug for NodejsRpcTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcTransformerPlugin({})", self.plugin_node.package_name)
  }
}

#[derive(Debug, Deserialize, Hash, PartialEq)]
#[serde(deny_unknown_fields)]
struct TransformerSetup {
  conditions: Option<SerializableTransformerConditions>,
  config: Option<JSONObject>,
  env: Option<JSONObject>,
}

async fn setup_plugin(
  nodejs_workers: Arc<NodeJsWorkerCollection>,
  rpc_options: RpcPluginOptions,
  plugin_node: &PluginNode,
) -> anyhow::Result<Option<TransformerSetup>> {
  let mut set = vec![];

  let opts = LoadPluginOptions {
    kind: LoadPluginKind::Transformer,
    specifier: plugin_node.package_name.clone(),
    resolve_from: plugin_node.resolve_from.as_ref().clone(),
    options: rpc_options,
  };

  for worker in nodejs_workers.all_workers() {
    let opts = opts.clone();
    set.push(tokio::spawn(async move {
      worker
        .load_plugin_fn
        .call_serde::<LoadPluginOptions, Option<TransformerSetup>>(opts)
        .await
    }));
  }

  let mut transformer_setup = None;

  while let Some(res) = set.pop() {
    let result = res.await??;

    if let Some(existing) = &transformer_setup {
      if existing != &result {
        return Err(anyhow::anyhow!(
          "Plugin {} returned inconsistent state from multiple workers",
          plugin_node.package_name
        ));
      }
    } else {
      transformer_setup = Some(result);
    }
  }

  transformer_setup.ok_or(anyhow::anyhow!(
    "Plugin {} did complete setup correctly",
    plugin_node.package_name
  ))
}

impl NodejsRpcTransformerPlugin {
  #[tracing::instrument(level = "info", skip_all, fields(plugin = %plugin_node.package_name))]
  pub async fn new(
    nodejs_workers: Arc<NodeJsWorkerCollection>,
    ctx: &PluginContext,
    plugin_node: &PluginNode,
    package_manager: PackageManagerRef,
  ) -> Result<Self, anyhow::Error> {
    // Calculate Dev dep hash on seperate thread
    let dev_dep_hash = task::spawn_blocking({
      let plugin_node = plugin_node.clone();
      move || {
        package_manager.resolve_dev_dependency(&plugin_node.package_name, &plugin_node.resolve_from)
      }
    });

    let rpc_options = RpcPluginOptions {
      hmr_options: ctx.options.hmr_options.clone(),
      project_root: ctx.options.project_root.clone(),
      mode: ctx.options.mode.clone(),
      feature_flags: ctx.options.feature_flags.clone(),
    };

    let transformer_setup = setup_plugin(nodejs_workers.clone(), rpc_options, plugin_node).await?;

    let (conditions, cache_key) = if let Some(setup) = transformer_setup {
      let cache_key = match dev_dep_hash.await??.hash {
        Some(dev_dep_hash) => {
          let mut hasher = IdentifierHasher::new();

          setup.hash(&mut hasher);
          hasher.write_u64(dev_dep_hash);

          CacheStatus::Hash(hasher.finish())
        }
        None => CacheStatus::Uncachable,
      };

      let conditions = Conditions::try_from(setup.conditions)?;
      tracing::debug!("Loaded transformer with conditions: {:?}", conditions);

      (conditions, cache_key)
    } else {
      tracing::warn!(
        "Plugin {} did not return any setup information, defaulting to no conditions and uncachable",
        plugin_node.package_name
      );
      (Conditions::default(), CacheStatus::Uncachable)
    };

    Ok(Self {
      nodejs_workers,
      plugin_options: ctx.options.clone(),
      plugin_node: plugin_node.clone(),
      conditions,
      cache_key,
    })
  }
}

#[async_trait]
impl TransformerPlugin for NodejsRpcTransformerPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.plugin_node.hash(&mut hasher);
    hasher.finish()
  }

  #[tracing::instrument(level = "trace", skip_all)]
  fn should_skip(&self, asset: &Asset) -> anyhow::Result<bool> {
    self.conditions.should_skip(asset)
  }

  #[tracing::instrument(level = "debug", skip_all, fields(plugin = %self.plugin_node.package_name))]
  fn cache_key(&self) -> &CacheStatus {
    &self.cache_key
  }

  async fn transform(
    &self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let asset_env = asset.env.clone();
    let stats = asset.stats.clone();

    let original_source_map = asset.map.clone();
    let source_map = if let Some(map) = asset.map.as_ref() {
      Some(map.to_json()?)
    } else {
      None
    };

    let run_transformer_opts = RpcTransformerOpts {
      key: self.plugin_node.package_name.clone(),
      env: asset_env.clone(),
      asset: Asset {
        code: Default::default(),
        ..asset.clone()
      },
    };

    let (result, contents, map, cache_bailout) = self
      .nodejs_workers
      .next_worker()
      .transformer_register_fn
      .call(
        move |env| {
          let run_transformer_opts = env.to_js_value(&run_transformer_opts)?;

          // Passing in an empty buffer causes napi to panic. If asset has no
          // code, we pass an empty Vec instead.
          let contents = if asset.code.is_empty() {
            env.create_buffer_with_data(Vec::new())?
          } else {
            let mut contents = env.create_buffer(asset.code.len())?;
            contents.copy_from_slice(&asset.code);
            contents
          };

          let map = if let Some(map) = source_map {
            env.create_string(&map)?.into_unknown()
          } else {
            env.get_undefined()?.into_unknown()
          };

          Ok(vec![run_transformer_opts, contents.into_unknown(), map])
        },
        |env, return_value| {
          let return_value = JsObject::from_unknown(return_value)?;

          let transform_result = return_value.get_element::<JsUnknown>(0)?;
          let transform_result = env.from_js_value::<RpcAssetResult, _>(transform_result)?;

          let contents = return_value.get_element::<JsUnknown>(1)?;

          // If there was nothing returned from an asset transform, then the
          // buffer will be empty, which causes napi to panic. If it is empty,
          // the worker will return `null` instead of an empty buffer, which
          // means we can check for it here and use an empty Vec.
          let contents = if contents.is_buffer()? {
            JsBuffer::from_unknown(contents)?.into_value()?.to_vec()
          } else {
            vec![]
          };

          let map = return_value.get_element::<JsString>(2)?.into_utf8()?;
          let map = if map.is_empty() {
            None
          } else {
            Some(map.into_owned()?)
          };

          let cacheable = return_value.get_element::<JsUnknown>(3)?;
          let cacheable = env.from_js_value::<bool, _>(cacheable)?;

          Ok((transform_result, contents, map, cacheable))
        },
      )
      .await
      .with_context(|| {
        format!(
          "Failed to transform asset '{}' with transformer '{}'",
          asset.file_path.display(),
          self.plugin_node.package_name
        )
      })?;

    let transformed_asset = Asset {
      id: result.id,
      code: Code::new(contents),
      bundle_behavior: result.bundle_behavior,
      env: asset_env.clone(),
      file_path: result.file_path,
      file_type: result.file_type,
      map: if let Some(json) = map {
        Some(SourceMap::from_json(
          &self.plugin_options.project_root,
          &json,
        )?)
      } else {
        original_source_map
      },
      meta: result.meta,
      pipeline: result.pipeline,
      query: result.query,
      stats,
      symbols: result.symbols,
      unique_key: result.unique_key,
      side_effects: result.side_effects,
      is_bundle_splittable: result.is_bundle_splittable,
      is_source: result.is_source,
      ..asset
    };

    Ok(TransformResult {
      // Adding dependencies from Node plugins isn't yet supported
      // TODO: Handle invalidations
      asset: transformed_asset,
      cache_bailout,
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
  pub env: Arc<Environment>,
  pub asset: Asset,
}
