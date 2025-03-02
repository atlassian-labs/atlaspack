use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Entry;
use atlaspack_filesystem::FileSystemRef;
use petgraph::graph::NodeIndex;

use crate::cache::KVCache;
use crate::plugins::PluginsRef;

pub struct Compilation {
  pub cache: KVCache,
  pub fs: FileSystemRef,
  pub options: Arc<AtlaspackOptions>,
  pub config_loader: ConfigLoaderRef,
  pub plugins: PluginsRef,
  pub project_root: PathBuf,
  pub asset_graph: AssetGraph,
  pub entries: Vec<Entry>,
  pub entry_dependencies: Vec<(NodeIndex, Arc<Dependency>)>,
}
