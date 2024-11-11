use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use atlaspack_config::atlaspack_rc_config_loader::AtlaspackRcConfigLoader;
use atlaspack_config::atlaspack_rc_config_loader::LoadConfigOptions;
use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::asset_graph::AssetNode;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::PluginLogger;
use atlaspack_core::plugin::PluginOptions;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_filesystem::os_file_system::OsFileSystem;
use atlaspack_filesystem::FileSystemRef;
use atlaspack_package_manager::NodePackageManager;
use atlaspack_package_manager::PackageManagerRef;
use atlaspack_plugin_rpc::RpcFactoryRef;
use lmdb_js_lite::writer::DatabaseWriter;
use lmdb_js_lite::writer::DatabaseWriterError;
use tokio::runtime::Runtime;

use crate::plugins::config_plugins::ConfigPlugins;
use crate::plugins::PluginsRef;
use crate::project_root::infer_project_root;
use crate::request_tracker::RequestTracker;
use crate::requests::AssetGraphRequest;
use crate::requests::RequestResult;

#[derive(Clone)]
struct AtlaspackState {
  config: Arc<ConfigLoader>,
  plugins: PluginsRef,
}

pub struct Atlaspack {
  pub db: Arc<DatabaseWriter>,
  pub fs: FileSystemRef,
  pub options: AtlaspackOptions,
  pub package_manager: PackageManagerRef,
  pub project_root: PathBuf,
  pub rpc: RpcFactoryRef,
  pub runtime: Runtime,
}

impl Atlaspack {
  pub fn new(
    db: Arc<DatabaseWriter>,
    fs: Option<FileSystemRef>,
    options: AtlaspackOptions,
    package_manager: Option<PackageManagerRef>,
    rpc: RpcFactoryRef,
  ) -> Result<Self, anyhow::Error> {
    let fs = fs.unwrap_or_else(|| Arc::new(OsFileSystem));
    let project_root = infer_project_root(Arc::clone(&fs), options.entries.clone())?;

    let package_manager = package_manager
      .unwrap_or_else(|| Arc::new(NodePackageManager::new(project_root.clone(), fs.clone())));

    let threads = num_cpus::get();
    tracing::info!(%threads, "Creating tokio runtime...");
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .thread_name("atlaspack-tokio-worker")
      .worker_threads(threads)
      .build()?;

    Ok(Self {
      db,
      fs,
      options,
      package_manager,
      project_root,
      rpc,
      runtime,
    })
  }
}

pub struct BuildResult;

impl Atlaspack {
  fn state(&self) -> anyhow::Result<AtlaspackState> {
    let rpc_worker = self.rpc.start()?;

    let (config, _files) =
      AtlaspackRcConfigLoader::new(Arc::clone(&self.fs), Arc::clone(&self.package_manager)).load(
        &self.project_root,
        LoadConfigOptions {
          additional_reporters: vec![], // TODO
          config: self.options.config.as_deref(),
          fallback_config: self.options.fallback_config.as_deref(),
        },
      )?;

    let config_loader = Arc::new(ConfigLoader {
      fs: Arc::clone(&self.fs),
      project_root: self.project_root.clone(),
      search_path: self.project_root.join("index"),
    });

    let plugins = Arc::new(ConfigPlugins::new(
      rpc_worker.clone(),
      config,
      PluginContext {
        config: Arc::clone(&config_loader),
        file_system: self.fs.clone(),
        options: Arc::new(PluginOptions {
          core_path: self.options.core_path.clone(),
          env: self.options.env.clone(),
          log_level: self.options.log_level.clone(),
          mode: self.options.mode.clone(),
          project_root: self.project_root.clone(),
        }),
        // TODO Initialise actual logger
        logger: PluginLogger::default(),
      },
    )?);

    let state = AtlaspackState {
      config: config_loader,
      plugins,
    };

    Ok(state)
  }

  pub fn build_asset_graph(&self) -> anyhow::Result<AssetGraph> {
    self.runtime.block_on(async move {
      let AtlaspackState { config, plugins } = self.state().unwrap();

      let mut request_tracker = RequestTracker::new(
        config.clone(),
        self.fs.clone(),
        Arc::new(self.options.clone()),
        plugins.clone(),
        self.project_root.clone(),
      );

      let state = request_tracker.state().clone();
      let reporter_handle = tokio::spawn(async move {
        loop {
          {
            let state = state.lock().await;
            let stats = state.get_request_stats();
            tracing::info!("Request stats: {:?}", stats);
          }
          tokio::time::sleep(Duration::from_secs(1)).await;
        }
      });

      let request_result = request_tracker.run_request(AssetGraphRequest {}).await?;

      let asset_graph = match &*request_result {
        RequestResult::AssetGraph(result) => {
          self.commit_assets(result.graph.assets.as_slice()).unwrap();
          // this is wasteful
          result.graph.clone()
        }
        _ => panic!("TODO"),
      };

      reporter_handle.abort();

      Ok(asset_graph)
    })
  }

  fn commit_assets(&self, assets: &[AssetNode]) -> Result<(), DatabaseWriterError> {
    let mut txn = self.db.environment().write_txn()?;

    for AssetNode { asset, .. } in assets.iter() {
      self.db.put(&mut txn, &asset.id, asset.code.bytes())?;
    }

    txn.commit()?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::{collections::HashSet, env::temp_dir};

  use atlaspack_core::types::{Asset, Code};
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
  use atlaspack_plugin_rpc::{MockRpcFactory, MockRpcWorker};
  use lmdb_js_lite::LMDBOptions;

  use super::*;

  #[test]
  fn build_asset_graph_commits_assets_to_lmdb() -> Result<(), anyhow::Error> {
    // TODO: Create overlay fs for integration test
    let db = Arc::new(create_db()?);
    let fs = InMemoryFileSystem::default();

    let atlaspack = Atlaspack::new(
      db.clone(),
      Some(Arc::new(fs)),
      AtlaspackOptions::default(),
      None,
      rpc(),
    )?;

    let assets = vec!["foo", "bar", "baz"];

    atlaspack.commit_assets(
      &assets
        .iter()
        .enumerate()
        .map(|(idx, asset)| AssetNode {
          asset: Asset {
            id: idx.to_string(),
            code: Code::from(asset.to_string()),
            ..Asset::default()
          },
          requested_symbols: HashSet::new(),
        })
        .collect::<Vec<AssetNode>>(),
    )?;

    let txn = db.environment().read_txn()?;
    for (idx, asset) in assets.iter().enumerate() {
      let entry = db.get(&txn, &idx.to_string())?;
      assert_eq!(entry, Some(asset.to_string().into()));
    }

    Ok(())
  }

  fn create_db() -> Result<DatabaseWriter, DatabaseWriterError> {
    let path = temp_dir().join("atlaspack").join("asset-graph-tests");
    let _ = std::fs::remove_dir_all(&path);

    let lmdb = DatabaseWriter::new(&LMDBOptions {
      path: path.to_string_lossy().to_string(),
      async_writes: false,
      map_size: None,
    })?;

    Ok(lmdb)
  }

  fn rpc() -> RpcFactoryRef {
    let mut rpc_factory = MockRpcFactory::new();

    rpc_factory
      .expect_start()
      .returning(|| Ok(Arc::new(MockRpcWorker::new())));

    Arc::new(rpc_factory)
  }
}
