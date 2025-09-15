use std::future::Future;
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
use tracing::warn;

use crate::plugins::{config_plugins::ConfigPlugins, PluginsRef};
use crate::project_root::infer_project_root;
use crate::request_tracker::{self, RequestTracker};
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
  /// Set to None if we were already running in an asynchronous context.
  pub runtime: Option<Runtime>,
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

    let runtime = create_tokio_runtime(&options)?;

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

  #[deprecated(note = "Methods need to return futures")]
  fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
    if let Some(runtime) = &self.runtime {
      runtime.block_on(future)
    } else {
      warn!("Blocking on an asynchronous context. This will likely crash.");
      tokio::runtime::Handle::current().block_on(future)
    }
  }
}

impl Atlaspack {
  #[tracing::instrument(skip(self, request))]
  pub async fn run_request_async(
    &self,
    request: impl request_tracker::Request,
  ) -> anyhow::Result<RequestResult> {
    self
      .request_tracker
      .write()
      .await
      .run_request(request)
      .await
  }

  /// Running this from a tokio thread will cause a crash.
  pub fn run_request(
    &self,
    request: impl request_tracker::Request,
  ) -> anyhow::Result<RequestResult> {
    #[allow(deprecated)]
    self.block_on(self.run_request_async(request))
  }

  pub async fn build_asset_graph_async(&self) -> anyhow::Result<AssetGraph> {
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
  }

  pub fn build_asset_graph(&self) -> anyhow::Result<AssetGraph> {
    #[allow(deprecated)]
    let result = self.block_on(self.build_asset_graph_async());

    if let Err(e) = &result {
      tracing::error!(?e, "Error building asset graph");
    }

    result
  }

  #[deprecated(note = "Methods need to return futures")]
  pub fn respond_to_fs_events(&self, events: WatchEvents) -> anyhow::Result<bool> {
    #[allow(deprecated)]
    self.block_on(async move {
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
        .put(&mut txn, &asset.id_string(), asset.code.bytes())?;
      if let Some(map) = &asset.map {
        // TODO: For some reason to_buffer strips data when rkyv was upgraded, so now we use json
        self.db.database().put(
          &mut txn,
          &format!("map:{}", asset.id_string()),
          map.to_json()?.as_bytes(),
        )?;
      }
    }

    txn.commit()?;

    Ok(())
  }
}

/// Create a tokio runtime if running in a synchronous context.
fn create_tokio_runtime(options: &AtlaspackOptions) -> anyhow::Result<Option<Runtime>> {
  if tokio::runtime::Handle::try_current().is_ok() {
    Ok(None)
  } else {
    Ok(Some(
      tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(options.threads.unwrap_or_else(num_cpus::get))
        .build()?,
    ))
  }
}

#[cfg(test)]
mod tests;
