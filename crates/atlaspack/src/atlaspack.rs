use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use atlaspack_config::atlaspack_rc_config_loader::{AtlaspackRcConfigLoader, LoadConfigOptions};
use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode, AssetNode};
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::plugin::{PluginContext, PluginLogger, PluginOptions};
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_filesystem::{os_file_system::OsFileSystem, FileSystemRef};
use atlaspack_package_manager::{NodePackageManager, PackageManagerRef};
use atlaspack_plugin_rpc::{RpcFactoryRef, RpcWorkerRef};
use lmdb_js_lite::writer::DatabaseWriter;
use petgraph::graph::NodeIndex;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use crate::plugins::{config_plugins::ConfigPlugins, PluginsRef};
use crate::project_root::infer_project_root;
use crate::request_tracker::RequestTracker;
use crate::requests::{AssetGraphRequest, RequestResult};
use crate::WatchEvents;

pub struct AtlaspackInitOptions {
  pub db: Arc<DatabaseWriter>,
  pub fs: Option<FileSystemRef>,
  pub options: AtlaspackOptions,
  pub package_manager: Option<PackageManagerRef>,
  pub rpc: RpcFactoryRef,
}

pub struct Atlaspack {
  pub db: Arc<DatabaseWriter>,
  pub fs: FileSystemRef,
  pub options: AtlaspackOptions,
  pub package_manager: PackageManagerRef,
  pub project_root: PathBuf,
  pub rpc: RpcFactoryRef,
  pub rpc_worker: RpcWorkerRef,
  pub runtime: Runtime,
  pub config_loader: Arc<ConfigLoader>,
  pub plugins: PluginsRef,
  pub request_tracker: Arc<RwLock<RequestTracker>>,
}

impl Atlaspack {
  pub fn new(
    AtlaspackInitOptions {
      db,
      fs,
      options,
      package_manager,
      rpc,
    }: AtlaspackInitOptions,
  ) -> Result<Self, anyhow::Error> {
    let fs = fs.unwrap_or_else(|| Arc::new(OsFileSystem));
    let project_root = infer_project_root(Arc::clone(&fs), options.entries.clone())?;

    let package_manager = package_manager
      .unwrap_or_else(|| Arc::new(NodePackageManager::new(project_root.clone(), fs.clone())));

    let runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .worker_threads(options.threads.unwrap_or_else(num_cpus::get))
      .build()?;

    let rpc_worker = rpc.start()?;

    let rc_config_loader =
      AtlaspackRcConfigLoader::new(Arc::clone(&fs), Arc::clone(&package_manager));

    let (config, _files) = rc_config_loader.load(
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
          feature_flags: options.feature_flags.clone(),
        }),
        // TODO Initialise actual logger
        logger: PluginLogger::default(),
      },
    )?);

    let request_tracker = RequestTracker::new(
      config_loader.clone(),
      fs.clone(),
      Arc::new(options.clone()),
      plugins.clone(),
      project_root.clone(),
    );

    Ok(Self {
      db,
      fs,
      options,
      package_manager,
      project_root,
      rpc,
      rpc_worker,
      runtime,
      config_loader,
      plugins,
      request_tracker: Arc::new(RwLock::new(request_tracker)),
    })
  }
}

impl Atlaspack {
  pub fn build_asset_graph(&self) -> anyhow::Result<AssetGraph> {
    self.runtime.block_on(async move {
      let request_result = self
        .request_tracker
        .write()
        .await
        .run_request(AssetGraphRequest {})
        .await?;

      let RequestResult::AssetGraph(asset_graph_request_output) = request_result else {
        panic!("Something went wrong with the request tracker")
      };

      let asset_graph = asset_graph_request_output.graph;
      self.commit_assets(&asset_graph)?;

      Ok(asset_graph)
    })
  }

  pub fn respond_to_fs_events(&self, events: WatchEvents) -> anyhow::Result<bool> {
    self.runtime.block_on(async move {
      Ok(
        self
          .request_tracker
          .write()
          .await
          .respond_to_fs_events(events),
      )
    })
  }

  #[tracing::instrument(level = "info", skip_all)]
  fn commit_assets(&self, graph: &AssetGraph) -> anyhow::Result<()> {
    let mut txn = self.db.write_txn()?;

    for node in graph.nodes() {
      let AssetGraphNode::Asset(asset_node) = node else {
        continue;
      };
      if asset_node.cached {
        continue;
      }

      let asset = &asset_node.asset;

      self.db.put(&mut txn, &asset.id, asset.code.bytes())?;
      if let Some(map) = &asset.map {
        // TODO: For some reason to_buffer strips data when rkyv was upgraded, so now we use json
        self.db.put(
          &mut txn,
          &format!("map:{}", asset.id),
          map.to_json()?.as_bytes(),
        )?;
      }
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
  use lmdb_js_lite::{writer::DatabaseWriterError, LMDBOptions};

  use super::*;

  #[test]
  fn build_asset_graph_commits_assets_to_lmdb() -> Result<(), anyhow::Error> {
    // TODO: Create overlay fs for integration test
    let db = Arc::new(create_db()?);
    let fs = InMemoryFileSystem::default();

    fs.write_file(
      &PathBuf::from("/.parcelrc"),
      r#"
      {
        "bundler": "",
        "namers": [""],
        "resolvers": [""],
      }
    "#
      .to_string(),
    );

    let atlaspack = Atlaspack::new(AtlaspackInitOptions {
      db: db.clone(),
      fs: Some(Arc::new(fs)),
      options: AtlaspackOptions::default(),
      package_manager: None,
      rpc: rpc(),
    })?;

    let assets_names = ["foo", "bar", "baz"];
    let mut asset_graph = AssetGraph::new();
    assets_names.iter().enumerate().for_each(|(idx, asset)| {
      asset_graph.add_asset(
        Asset {
          id: idx.to_string(),
          code: Code::from(asset.to_string()),
          ..Asset::default()
        },
        true,
      );
    });

    atlaspack.commit_assets(&asset_graph)?;

    let txn = db.read_txn()?;
    for (idx, asset) in assets_names.iter().enumerate() {
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
