use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_config::atlaspack_rc_config_loader::AtlaspackRcConfigLoader;
use atlaspack_config::atlaspack_rc_config_loader::LoadConfigOptions;
use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::PluginLogger;
use atlaspack_core::plugin::PluginOptions;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Entry;
use atlaspack_filesystem::FileSystemRef;

use atlaspack_package_manager::PackageManagerRef;
use atlaspack_plugin_rpc::RpcFactoryRef;
use petgraph::graph::NodeIndex;

use crate::cache::KVCache;
use crate::plugins::config_plugins::ConfigPlugins;
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
  pub entry_dependencies: Vec<(NodeIndex, Dependency)>,
}

impl Compilation {
  pub async fn new(
    CompilationOptions {
      rpc,
      project_root,
      options,
      fs,
      package_manager,
      cache,
      asset_graph,
      entries,
      entry_dependencies,
    }: CompilationOptions,
  ) -> anyhow::Result<Self> {
    let rpc_worker = rpc.start()?;

    let (config, _files) =
      AtlaspackRcConfigLoader::new(Arc::clone(&fs), Arc::clone(&package_manager)).load(
        &project_root,
        LoadConfigOptions {
          additional_reporters: vec![], // TODO
          config: options.config.as_deref(),
          fallback_config: options.fallback_config.as_deref(),
        },
      )?;

    let config_loader = Arc::new(ConfigLoader {
      fs: Arc::clone(&fs),
      project_root: project_root.clone(),
      search_path: project_root.join("index"),
    });

    let plugins = Arc::new(ConfigPlugins::new(
      rpc_worker.clone(),
      config,
      PluginContext {
        config: Arc::clone(&config_loader),
        file_system: fs.clone(),
        options: Arc::new(PluginOptions {
          core_path: options.core_path.clone(),
          env: options.env.clone(),
          log_level: options.log_level.clone(),
          mode: options.mode.clone(),
          project_root: project_root.clone(),
        }),
        // TODO Initialise actual logger
        logger: PluginLogger::default(),
      },
    )?);

    Ok(Compilation {
      asset_graph,
      cache,
      config_loader,
      entries,
      entry_dependencies,
      fs,
      options,
      plugins,
      project_root,
    })
  }
}

pub struct CompilationOptions {
  pub cache: KVCache,
  pub rpc: RpcFactoryRef,
  pub fs: FileSystemRef,
  pub package_manager: PackageManagerRef,
  pub options: Arc<AtlaspackOptions>,
  pub project_root: PathBuf,
  pub asset_graph: AssetGraph,
  pub entries: Vec<Entry>,
  pub entry_dependencies: Vec<(NodeIndex, Dependency)>,
}
