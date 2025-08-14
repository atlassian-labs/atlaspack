use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_config::atlaspack_rc_config_loader::{AtlaspackRcConfigLoader, LoadConfigOptions};
use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode, AssetNode};
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::plugin::{PluginContext, PluginLogger, PluginOptions};
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_filesystem::{os_file_system::OsFileSystem, FileSystemRef};
use atlaspack_package_manager::{NodePackageManager, PackageManagerRef};
use atlaspack_plugin_rpc::{RpcFactoryRef, RpcWorkerRef};
use lmdb_js_lite::DatabaseHandle;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use crate::plugins::{config_plugins::ConfigPlugins, PluginsRef};
use crate::project_root::infer_project_root;
use crate::request_tracker::RequestTracker;
use crate::requests::{AssetGraphRequest, RequestResult};
use crate::WatchEvents;

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
    let project_root = infer_project_root(Arc::clone(&fs), options.entries.clone())?;
    tracing::debug!(?project_root, entries = ?options.entries, "Project root");

    let package_manager = package_manager
      .unwrap_or_else(|| Arc::new(NodePackageManager::new(project_root.clone(), fs.clone())));

    let runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .worker_threads(options.threads.unwrap_or_else(num_cpus::get))
      .build()?;

    let rpc_worker = rpc.start()?;

    let rc_config_loader =
      AtlaspackRcConfigLoader::new(Arc::clone(&fs), Arc::clone(&package_manager));

    tracing::debug!(?project_root, "Loading config");
    let (config, _files) = rc_config_loader.load(
      &project_root,
      LoadConfigOptions {
        additional_reporters: vec![], // TODO
        config: options.config.as_deref(),
        fallback_config: options.fallback_config.as_deref(),
      },
    )?;

    let config_loader = Arc::new(ConfigLoader::new(
      Arc::clone(&fs),
      project_root.clone(),
      project_root.join("index"),
    ));

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
    let result = self.runtime.block_on(async move {
      let request_result = self
        .request_tracker
        .write()
        .await
        .run_request(AssetGraphRequest {})
        .await?;

      let RequestResult::AssetGraph(asset_graph_request_output) = request_result else {
        anyhow::bail!("Something went wrong with the request tracker")
      };

      let asset_graph = asset_graph_request_output.graph;
      self.commit_assets(asset_graph.nodes().collect())?;
      Ok(asset_graph)
    });

    if let Err(e) = &result {
      tracing::error!(?e, "Error building asset graph");
    }

    result
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

  fn commit_assets(&self, assets: Vec<&AssetGraphNode>) -> anyhow::Result<()> {
    let mut txn = self.db.database().write_txn()?;

    for asset_node in assets {
      let AssetGraphNode::Asset(AssetNode { asset, .. }) = asset_node else {
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
  use std::{
    collections::{HashMap, HashSet},
    env::temp_dir,
    path::Path,
  };

  use atlaspack_benchmark::GenerateMonorepoParams;
  use atlaspack_core::types::{Asset, Code};
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
  use atlaspack_plugin_rpc::{rust::RustWorkerFactory, MockRpcFactory, MockRpcWorker};
  use lmdb_js_lite::{get_database, LMDBOptions};

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
    let assets = assets_names
      .iter()
      .enumerate()
      .map(|(idx, asset)| {
        AssetGraphNode::Asset(AssetNode {
          asset: Asset {
            id: idx.to_string(),
            code: Code::from(asset.to_string()),
            ..Asset::default()
          },
          requested_symbols: HashSet::new(),
        })
      })
      .collect::<Vec<AssetGraphNode>>();

    atlaspack.commit_assets(assets.iter().collect())?;

    let txn = db.database().read_txn()?;
    for (idx, asset) in assets_names.iter().enumerate() {
      let entry = db.database().get(&txn, &idx.to_string())?;
      assert_eq!(entry, Some(asset.to_string().into()));
    }

    Ok(())
  }

  fn symlink_core(temp_dir: &Path) -> anyhow::Result<()> {
    let repo_path = get_repo_path();

    let do_link = |source, target| -> anyhow::Result<()> {
      let source_path = repo_path.join(source);
      let target_path = temp_dir.join(target);

      tracing::debug!(?source_path, ?target_path, "Linking");

      std::fs::create_dir_all(&target_path.parent().unwrap())?;
      std::os::unix::fs::symlink(&source_path, &target_path)?;

      Ok(())
    };

    do_link(
      "packages/configs/default",
      "node_modules/@atlaspack/config-default",
    )?;

    Ok(())
  }

  #[test]
  fn test_create_asset_graph_from_simple_app() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
      .with_max_level(tracing::Level::INFO)
      .try_init();

    let temp_dir = temp_dir()
      .join("atlaspack")
      .join("test-create-asset-graph-from-simple-app");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir)?;
    std::fs::create_dir_all(temp_dir.join("src"))?;
    let temp_dir = std::fs::canonicalize(&temp_dir)?;
    symlink_core(&temp_dir)?;

    std::fs::write(temp_dir.join("yarn.lock"), r#"{}"#).unwrap();
    std::fs::write(
      temp_dir.join(".parcelrc"),
      r#"{"extends": "@atlaspack/config-default"}"#,
    )
    .unwrap();

    std::fs::write(
      temp_dir.join("src/index.ts"),
      r#"
      import { bar } from "./bar";

      output(bar + "foo");
      "#,
    )
    .unwrap();

    std::fs::write(
      temp_dir.join("src/bar.ts"),
      r#"
      export const bar = "bar";
      "#,
    )
    .unwrap();

    let atlaspack = Atlaspack::new(AtlaspackInitOptions {
      db: create_db()?,
      fs: Some(Arc::new(OsFileSystem)),
      options: AtlaspackOptions {
        entries: vec![temp_dir.join("src/index.ts").to_string_lossy().to_string()],
        core_path: get_core_path(),
        ..Default::default()
      },
      package_manager: None,
      rpc: Arc::new(RustWorkerFactory::default()),
    })?;

    let asset_graph = atlaspack.build_asset_graph()?;

    tracing::debug!("Asset graph: {:#?}", asset_graph);

    let assets = asset_graph
      .nodes()
      .filter_map(|node| {
        if let AssetGraphNode::Asset(node) = node {
          Some(node)
        } else {
          None
        }
      })
      .collect::<Vec<_>>();

    let assets = assets
      .iter()
      .map(|asset| (asset.asset.file_path.clone(), asset.asset.clone()))
      .collect::<HashMap<_, _>>();

    tracing::debug!(test_path = ?get_repo_path().join("packages/transformers/js/src/esmodule-helpers.js"), "Assets");
    tracing::debug!(assets = ?assets.keys().collect::<Vec<_>>(), "Assets");

    assert_eq!(assets.len(), 3);
    assert!(assets.contains_key(&temp_dir.join("src/bar.ts")));
    assert!(assets.contains_key(&temp_dir.join("src/index.ts")));
    assert!(assets
      .contains_key(&get_repo_path().join("packages/transformers/js/src/esmodule-helpers.js")));

    Ok(())
  }

  #[test]
  fn test_build_asset_graph_from_large_synthetic_project() {
    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
      .with_max_level(tracing::Level::INFO)
      .try_init();

    let temp_dir = temp_dir()
      .join("atlaspack")
      .join("test-build-asset-graph-from-large-synthetic-project");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_dir = std::fs::canonicalize(&temp_dir).unwrap();

    atlaspack_benchmark::generate_monorepo(GenerateMonorepoParams {
      num_files: 100_000,
      avg_lines_per_file: 30,
      depth: 50_000,
      subtrees: 10_000,
      target_dir: &temp_dir,
      ..Default::default()
    })
    .unwrap();
    std::fs::write(temp_dir.join("yarn.lock"), r#"{}"#).unwrap();
    std::fs::write(
      temp_dir.join(".parcelrc"),
      r#"{"extends": "@atlaspack/config-default"}"#,
    )
    .unwrap();
    symlink_core(&temp_dir).unwrap();

    let atlaspack = Atlaspack::new(AtlaspackInitOptions {
      db: create_db().unwrap(),
      fs: Some(Arc::new(OsFileSystem)),
      options: AtlaspackOptions {
        entries: vec![temp_dir
          .join("app-root/src/index.ts")
          .to_string_lossy()
          .to_string()],
        core_path: get_core_path(),
        ..Default::default()
      },
      package_manager: None,
      rpc: Arc::new(RustWorkerFactory::default()),
    })
    .unwrap();

    let asset_graph = atlaspack.build_asset_graph().unwrap();

    let _ = std::fs::remove_dir_all(&temp_dir);
  }

  fn get_repo_path() -> PathBuf {
    let result = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&result).unwrap()
  }

  fn get_core_path() -> PathBuf {
    let result = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/core");
    std::fs::canonicalize(&result).unwrap()
  }

  fn create_db() -> anyhow::Result<Arc<DatabaseHandle>> {
    let path = temp_dir().join("atlaspack").join("asset-graph-tests");
    let _ = std::fs::remove_dir_all(&path);

    let lmdb = get_database(LMDBOptions {
      path: path.to_string_lossy().to_string(),
      async_writes: false,
      map_size: Some((1024 * 1024 * 1024usize) as f64),
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
