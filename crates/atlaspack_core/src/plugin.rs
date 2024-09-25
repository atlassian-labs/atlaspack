use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_filesystem::FileSystemRef;
pub use bundler_plugin::*;
pub use compressor_plugin::*;
pub use namer_plugin::*;
pub use optimizer_plugin::*;
pub use packager_plugin::*;
pub use reporter_plugin::*;
pub use resolver_plugin::*;
pub use runtime_plugin::*;
pub use transformer_plugin::*;
pub use validator_plugin::*;

use crate::config_loader::{ConfigLoader, ConfigLoaderRef};
use crate::types::{BuildMode, LogLevel};

mod bundler_plugin;
mod compressor_plugin;
mod namer_plugin;
mod optimizer_plugin;
mod packager_plugin;
mod reporter_plugin;
mod resolver_plugin;
mod runtime_plugin;
mod transformer_plugin;
mod validator_plugin;

pub struct PluginContext {
  pub config: ConfigLoaderRef,
  pub file_system: FileSystemRef,
  pub logger: PluginLogger,
  pub options: Arc<PluginOptions>,
}

#[derive(Default)]
pub struct PluginLogger {}

#[derive(Debug, Default)]
pub struct PluginOptions {
  pub core_path: PathBuf,
  /// Environment variables
  pub env: Option<HashMap<String, String>>,
  pub log_level: LogLevel,
  pub mode: BuildMode,
  pub project_root: PathBuf,
}

impl PluginOptions {
  pub fn should_scope_hoist(&self) -> bool {
    self.mode == BuildMode::Production
  }
}
