use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_filesystem::FileSystemRef;
pub use resolver_plugin::*;
use serde::{Deserialize, Serialize};
pub use transformer_plugin::*;

use crate::config_loader::ConfigLoaderRef;
use crate::types::{AliasMap, BuildMode, FeatureFlags, LogLevel};

mod resolver_plugin;
mod transformer_plugin;

pub struct PluginContext {
  pub config: ConfigLoaderRef,
  pub file_system: FileSystemRef,
  pub logger: PluginLogger,
  pub options: Arc<PluginOptions>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Hash)]
pub struct HmrOptions {
  pub port: Option<u32>,
  pub host: Option<String>,
}

#[derive(Default)]
pub struct PluginLogger {}

#[derive(Debug, Default)]
pub struct PluginOptions {
  pub core_path: PathBuf,
  /// Environment variables
  pub env: BTreeMap<String, String>,
  pub log_level: LogLevel,
  pub mode: BuildMode,
  pub project_root: PathBuf,
  pub feature_flags: FeatureFlags,
  pub hmr_options: Option<HmrOptions>,
  pub unstable_alias: Option<AliasMap>,
}
