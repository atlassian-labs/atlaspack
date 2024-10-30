use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::types::BuildMode;
use atlaspack_core::types::DefaultTargetOptions;
use atlaspack_core::types::LogLevel;
use atlaspack_filesystem::FileSystemRef;
use atlaspack_package_manager::PackageManagerRef;
use atlaspack_plugin_rpc::RpcFactoryRef;
use petgraph::graph::NodeIndex;
use tokio::sync::RwLock;

use crate::plugins::config_plugins::ConfigPlugins;
use crate::state::EnvMap;
use crate::AtlaspackOptions;

pub struct Compilation {
  pub options: Arc<AtlaspackOptions>,
  pub fs: FileSystemRef,
  pub package_manager: PackageManagerRef,
  pub project_root: PathBuf,
  pub log_level: LogLevel,
  pub mode: BuildMode,
  pub rpc: RpcFactoryRef,
  pub config_loader: Arc<ConfigLoader>,
  pub plugins: Arc<ConfigPlugins>,
  pub env: EnvMap,
  pub entries: Vec<String>,
  pub default_target_options: DefaultTargetOptions,
  pub asset_graph: Arc<RwLock<AssetGraph>>,
  pub asset_request_to_asset: Arc<RwLock<HashMap<u64, NodeIndex>>>,
  // waiting_asset_requests: HashMap<u64, HashSet<NodeIndex>>,
}
