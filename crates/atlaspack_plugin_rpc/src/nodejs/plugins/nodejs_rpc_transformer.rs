use async_trait::async_trait;
use atlaspack_core::plugin::PluginOptions;
use napi::bindgen_prelude::FromNapiValue;
use napi::JsBuffer;
use napi::JsObject;
use napi::JsString;
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

use crate::javascript_plugin_api::plugin_options::RpcPluginOptions;
use crate::javascript_plugin_api::JavaScriptPluginAPI;
use crate::javascript_plugin_api::LoadPluginKind;
use crate::javascript_plugin_api::LoadPluginOptions;
use crate::javascript_plugin_api::RpcTransformerOpts;
use crate::nodejs::nodejs_rpc_worker_farm::NodeJsWorkerCollection;

/// Plugin state once initialized
struct InitializedState {
  rpc_plugin_options: RpcPluginOptions,
}

pub struct NodejsRpcTransformerPlugin {
  nodejs_workers: Arc<dyn JavaScriptPluginAPI + Send + Sync>,
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
    nodejs_workers: Arc<dyn JavaScriptPluginAPI + Send + Sync>,
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
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let state = self.get_or_init_state().await?;

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
      options: state.rpc_plugin_options.clone(),
      env: asset_env.clone(),
      asset: Asset {
        code: Default::default(),
        ..asset
      },
    };

    let (result, contents, map) = self
      .nodejs_workers
      .transform(asset.code, source_map, run_transformer_opts)
      .await?;

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
