use std::{path::PathBuf, sync::Arc};

use atlaspack_core::types::{
  engines::Engines, Asset, BundleBehavior, Code, EnvironmentContext, FileType, IncludeNodeModules,
  JSONObject, OutputFormat, SourceLocation, SourceType, Symbol,
};
use serde::{Deserialize, Serialize};

pub type RpcHostRef = Arc<dyn RpcHost>;
pub type RpcWorkerRef = Arc<dyn RpcWorker>;

pub trait RpcHost: Send + Sync {
  fn start(&self) -> anyhow::Result<RpcWorkerRef>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcHmrOptions {
  pub port: Option<u16>,
  pub host: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcPluginOptions {
  pub hmr_options: Option<RpcHmrOptions>,
  pub project_root: PathBuf,
}

/// This Asset mostly replicates the core Asset type however it only features
/// fields that can be modified by transformers
#[derive(Default, PartialEq, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAssetResult {
  pub id: String,
  pub bundle_behavior: BundleBehavior,
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
  pub resolve_from: String,
  pub specifier: String,
  pub options: RpcPluginOptions,
  pub asset: Asset,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcTransformerResult {
  pub asset: RpcAssetResult,
}

pub trait RpcWorker: Send + Sync {
  fn ping(&self) -> anyhow::Result<()>;
  fn run_transformer(&self, opts: RpcTransformerOpts) -> anyhow::Result<RpcTransformerResult>;
}
