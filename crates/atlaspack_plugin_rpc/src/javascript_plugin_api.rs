use async_trait::async_trait;
use atlaspack_core::{
  plugin::Resolved,
  types::{
    engines::Engines, Asset, BundleBehavior, Code, Dependency, Environment, EnvironmentContext,
    FileType, IncludeNodeModules, JSONObject, OutputFormat, SourceLocation, SourceType, Symbol,
  },
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

use crate::javascript_plugin_api::plugin_options::RpcPluginOptions;

/// Internal API to dispatch work into JavaScript backends.
///
/// Current implementations are:
///
/// * Dispatch work to a pool of Node.js workers threads, when running embedded in
///   Node.js
/// * Dispatch work to a pool of Node.js worker threads, created using embedded libnode through
///   alshdavid/edon
#[async_trait]
pub trait JavaScriptPluginAPI: Send + Sync {
  async fn resolve(&self, opts: RunResolverResolve) -> anyhow::Result<Resolved>;
  async fn transform(
    &self,
    code: Code,
    source_map: Option<String>,
    run_transformer_opts: RpcTransformerOpts,
  ) -> anyhow::Result<(RpcAssetResult, Vec<u8>, Option<String>)>;

  async fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()>;
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadPluginKind {
  Resolver,
  Transformer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResolverResolve {
  pub key: String,
  pub dependency: Dependency,
  pub specifier: String,
  pub pipeline: Option<String>,
  pub plugin_options: RpcPluginOptions,
}

pub mod plugin_options {
  use std::path::PathBuf;

  use atlaspack_core::types::BuildMode;
  use serde::{Deserialize, Serialize};

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
    pub mode: BuildMode,
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test_load_plugin() {}
}
