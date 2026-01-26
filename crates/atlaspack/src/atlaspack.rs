use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_config::atlaspack_rc_config_loader::{AtlaspackRcConfigLoader, LoadConfigOptions};
use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode};
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::plugin::{PluginContext, PluginLogger, PluginOptions};
use atlaspack_core::types::{AtlaspackOptions, SourceField, Targets};
use atlaspack_filesystem::{FileSystemRef, os_file_system::OsFileSystem};
use atlaspack_memoization_cache::{CacheHandler, CacheMode, LmdbCacheReaderWriter, StatsSnapshot};
use atlaspack_package_manager::{NodePackageManager, PackageManagerRef};
use atlaspack_plugin_rpc::{RpcFactoryRef, RpcWorkerRef};
use lmdb_js_lite::DatabaseHandle;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use crate::WatchEvents;
use crate::plugins::{PluginsRef, config_plugins::ConfigPlugins};
use crate::project_root::infer_project_root;
use crate::request_tracker::{DynCacheHandler, RequestNode, RequestTracker};
use crate::requests::{AssetGraphRequest, RequestResult};

pub struct AtlaspackInitOptions {
  pub db: Arc<DatabaseHandle>,
  pub fs: Option<FileSystemRef>,
  pub options: AtlaspackOptions,
  pub package_manager: Option<PackageManagerRef>,
  pub rpc: RpcFactoryRef,
}

pub struct Atlaspack {
  pub db: Arc<DatabaseHandle>,
  pub fs: FileSystemRef,
  pub options: AtlaspackOptions,
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

    // When allowExplicitTargetEntries is enabled and no entries are provided,
    // automatically derive entries from target sources
    let mut resolved_options = options;
    if resolved_options
      .feature_flags
      .bool_enabled("allowExplicitTargetEntries")
      && resolved_options.entries.is_empty()
      && let Some(Targets::CustomTarget(custom_targets)) = &resolved_options.targets
    {
      let mut target_sources = HashSet::new();

      for (_, target) in custom_targets.iter() {
        if let Some(source) = &target.source {
          match source {
            SourceField::Source(source_str) => {
              target_sources.insert(source_str.clone());
            }
            SourceField::Sources(sources) => {
              for source_str in sources {
                target_sources.insert(source_str.clone());
              }
            }
          }
        }
      }

      resolved_options.entries = target_sources.into_iter().collect();
    }

    let project_root = infer_project_root(Arc::clone(&fs), resolved_options.entries.clone())?;

    let package_manager = package_manager
      .unwrap_or_else(|| Arc::new(NodePackageManager::new(project_root.clone(), fs.clone())));

    let runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .worker_threads(resolved_options.threads.unwrap_or_else(num_cpus::get))
      .build()?;

    let rpc_worker = rpc.start()?;

    let rc_config_loader = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager.clone());

    let (config, _files) = rc_config_loader.load(
      &project_root,
      LoadConfigOptions {
        additional_reporters: vec![], // TODO
        config: resolved_options.config.as_deref(),
        fallback_config: resolved_options.fallback_config.as_deref(),
      },
    )?;

    let unstable_alias = config.unstable_alias.clone();

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
          core_path: resolved_options.core_path.clone(),
          env: resolved_options.env.clone(),
          log_level: resolved_options.log_level.clone(),
          mode: resolved_options.mode.clone(),
          project_root: project_root.clone(),
          feature_flags: resolved_options.feature_flags.clone(),
          hmr_options: resolved_options.hmr_options.clone(),
          unstable_alias,
        }),
        // TODO Initialise actual logger
        logger: PluginLogger::default(),
      },
      package_manager,
    )?);

    let cache_mode = if resolved_options.feature_flags.bool_enabled("v3Caching") {
      // Validate 1 in 1000 cache requests
      CacheMode::On(0.001)
    } else {
      CacheMode::Off
    };

    let request_tracker = RequestTracker::new(
      config_loader.clone(),
      fs.clone(),
      Arc::new(resolved_options.clone()),
      plugins.clone(),
      project_root.clone(),
      Arc::new(DynCacheHandler::Lmdb(CacheHandler::new(
        LmdbCacheReaderWriter::new(db.clone()),
        cache_mode,
      ))),
    );

    Ok(Self {
      db,
      fs,
      options: resolved_options,
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
  pub fn build_asset_graph(
    &self,
  ) -> anyhow::Result<(
    Arc<AssetGraph>,
    Arc<atlaspack_core::asset_graph::SymbolTracker>,
    bool,
  )> {
    self.runtime.block_on(async move {
      // Notify all resolver plugins that a new build is starting
      for resolver in self.plugins.resolvers()? {
        resolver.on_new_build();
      }

      let mut request_tracker = self.request_tracker.write().await;

      let prev_asset_graph = request_tracker
        .get_cached_request_result(AssetGraphRequest::default())
        .map(|result| {
          let RequestResult::AssetGraph(asset_graph_request_output) = result.as_ref() else {
            panic!("Something went wrong with the request tracker")
          };
          asset_graph_request_output.graph.clone()
        });

      let incrementally_bundled_assets = request_tracker
        .get_invalid_nodes()
        .try_fold(
          Vec::new(),
          |mut invalid_nodes, invalid_node| match invalid_node {
            RequestNode::Invalid(Some(result)) => match result.as_ref() {
              RequestResult::Asset(_) => {
                invalid_nodes.push(result.clone());
                Ok(invalid_nodes)
              }
              RequestResult::AssetGraph(_) => Ok(invalid_nodes),
              _ => Err(()),
            },
            _ => Err(()),
          },
        )
        .ok();

      let had_previous_graph = prev_asset_graph.is_some();

      let request_result = request_tracker
        .run_request(AssetGraphRequest {
          prev_asset_graph,
          incrementally_bundled_assets,
        })
        .await?;

      let RequestResult::AssetGraph(asset_graph_request_output) = request_result.as_ref() else {
        panic!("Something went wrong with the request tracker")
      };

      let asset_graph = asset_graph_request_output.graph.clone();
      let symbol_tracker = asset_graph_request_output.symbol_tracker.clone();

      Ok((asset_graph, symbol_tracker, had_previous_graph))
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

  /// Get cache statistics
  pub async fn complete_cache_session(&self) -> Option<StatsSnapshot> {
    match self.request_tracker.read().await.cache.complete_session() {
      Ok(stats) => Some(stats),
      Err(_) => {
        tracing::warn!("Failed to complete cache session. Cache potentially in an invalid state.");
        None
      }
    }
  }

  #[tracing::instrument(level = "info", skip_all)]
  pub fn commit_assets(&self, graph: &AssetGraph) -> anyhow::Result<()> {
    let mut txn = self.db.database().write_txn()?;

    let nodes = graph.new_nodes().chain(graph.updated_nodes());

    for node in nodes {
      let AssetGraphNode::Asset(asset) = node else {
        continue;
      };

      self
        .db
        .database()
        .put(&mut txn, &asset.id, asset.code.bytes())?;
      if let Some(map) = &asset.map {
        // TODO: For some reason to_buffer strips data when rkyv was upgraded, so now we use json
        self.db.database().put(
          &mut txn,
          &format!("map:{}", asset.id),
          map.clone().to_json(None)?.as_bytes(),
        )?;
      }
    }

    txn.commit()?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::env::temp_dir;

  use atlaspack_core::types::{Asset, Code};
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
  use atlaspack_plugin_rpc::{MockRpcFactory, MockRpcWorker};
  use lmdb_js_lite::{LMDBOptions, get_database};

  use super::*;

  #[test]
  fn build_asset_graph_commits_assets_to_lmdb() -> Result<(), anyhow::Error> {
    // TODO: Create overlay fs for integration test
    let db = create_db()?;
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
        Arc::new(Asset {
          id: idx.to_string(),
          code: Code::from(asset.to_string()),
          ..Asset::default()
        }),
        false,
      );
    });

    atlaspack.commit_assets(&asset_graph)?;

    let txn = db.database().read_txn()?;
    for (idx, asset) in assets_names.iter().enumerate() {
      let entry = db.database().get(&txn, &idx.to_string())?;
      assert_eq!(entry, Some(asset.to_string().into()));
    }

    Ok(())
  }

  fn create_db() -> anyhow::Result<Arc<DatabaseHandle>> {
    let path = temp_dir().join("atlaspack").join("asset-graph-tests");
    let _ = std::fs::remove_dir_all(&path);

    let lmdb = get_database(LMDBOptions {
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
