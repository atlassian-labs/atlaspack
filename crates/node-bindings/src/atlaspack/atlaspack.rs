use core::str;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use anyhow::anyhow;
use atlaspack::AtlaspackInitOptions;
use atlaspack::WatchEvents;
use lmdb_js_lite::writer::DatabaseWriter;
use lmdb_js_lite::LMDB;
use napi::Env;
use napi::JsFunction;
use napi::JsObject;
use napi::JsUnknown;
use napi_derive::napi;

use atlaspack::file_system::FileSystemRef;
use atlaspack::rpc::nodejs::NodejsRpcFactory;
use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack_napi_helpers::JsTransferable;
use atlaspack_package_manager::PackageManagerRef;

use super::atlaspack_cell::AtlaspackCell;
use super::file_system_napi::FileSystemNapi;
use super::napi_ext::NapiExt;
use super::package_manager_napi::PackageManagerNapi;
use super::serialize_asset_graph::serialize_asset_graph;

#[napi(object)]
pub struct AtlaspackNapiOptions {
  pub fs: Option<JsObject>,
  pub node_workers: Option<u32>,
  pub options: JsObject,
  pub package_manager: Option<JsObject>,
  pub threads: Option<u32>,
  pub register_worker: JsFunction,
  pub release_workers: JsFunction,
}

#[napi]
pub struct AtlaspackNapi {
  pub node_worker_count: u32,
  atlaspack: AtlaspackCell,
  release_workers: JsFunction,
}

// Refer to https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/length
const MAX_STRING_LENGTH: usize = 268435440;

#[napi]
impl AtlaspackNapi {
  #[tracing::instrument(level = "info", skip_all)]
  #[napi]
  pub fn create(napi_options: AtlaspackNapiOptions, lmdb: &LMDB, env: Env) -> napi::Result<Self> {
    let thread_id = std::thread::current().id();
    tracing::trace!(?thread_id, "atlaspack-napi initialize");

    // Wrap the JavaScript-supplied FileSystem
    let fs: Option<FileSystemRef> = if let Some(fs) = napi_options.fs {
      Some(Arc::new(FileSystemNapi::new(&env, &fs)?))
    } else {
      None
    };

    let package_manager: Option<PackageManagerRef> = if let Some(pm) = napi_options.package_manager
    {
      Some(Arc::new(PackageManagerNapi::new(&env, &pm)?))
    } else {
      None
    };

    let db_handle = lmdb.get_database_napi()?.clone();
    let db_writer = db_handle.database();

    Self::run_db_healthcheck(db_writer)?;

    let db = db_writer.clone();

    // Assign Rust thread count from JavaScript
    let threads = napi_options
      .threads
      .map(|t| t as usize)
      .unwrap_or_else(num_cpus::get);

    // Set up Nodejs plugin bindings
    let node_worker_count = napi_options
      .node_workers
      .map(|w| w as usize)
      .unwrap_or_else(|| threads);

    let (tx_worker, rx_worker) = channel::<NodejsWorker>();
    let rpc_host_nodejs = NodejsRpcFactory::new(node_worker_count, rx_worker)?;
    let rpc = Arc::new(rpc_host_nodejs);
    let options = env.from_js_value(napi_options.options)?;

    // Initialize Nodejs worker threads
    for _ in 0..node_worker_count {
      let transferable = JsTransferable::new(tx_worker.clone());

      napi_options
        .register_worker
        .call1::<JsTransferable<Sender<NodejsWorker>>, JsUnknown>(transferable)?;
    }

    // Initialize Atlaspack on another thread
    let atlaspack = AtlaspackCell::new(AtlaspackInitOptions {
      db,
      fs,
      options,
      package_manager,
      rpc,
    });

    Ok(Self {
      node_worker_count: node_worker_count as u32,
      atlaspack,
      release_workers: napi_options.release_workers,
    })
  }

  /// Shutdown Atlaspack and release worker threads
  #[napi]
  pub fn shutdown(&self, _env: Env) -> napi::Result<()> {
    self.release_workers.call0::<JsUnknown>()?;
    Ok(())
  }

  #[tracing::instrument(level = "info", skip_all)]
  #[napi]
  pub fn build_asset_graph(&self, env: Env) -> napi::Result<JsObject> {
    let atlaspack = self.atlaspack.clone();

    env.spawn_thread(move || {
      let atlaspack = atlaspack.get()?;
      let result = atlaspack.build_asset_graph()?;

      // Runs on Javascript Thread
      Ok(move |env: Env| Ok(serialize_asset_graph(&env, &result)?))
    })
  }

  #[tracing::instrument(level = "info", skip_all)]
  #[napi]
  pub fn respond_to_fs_events(&self, env: Env, options: JsObject) -> napi::Result<JsObject> {
    let options = env.from_js_value::<WatchEvents, _>(options)?;
    let atlaspack = self.atlaspack.clone();

    // Runs on separate thread
    env.spawn_thread(move || {
      let atlaspack = atlaspack.get()?;
      let result = atlaspack.respond_to_fs_events(options)?;

      // Runs on Javascript Thread
      Ok(move |_env: Env| Ok(result))
    })
  }

  /// Check that the LMDB database is healthy
  ///
  /// JavaScript does all its writes through a single thread, which is not this handle. If we want
  /// to sequence writes with the JavaScript writes, we should be using the
  /// [`lmdb_js_lite::writer::DatabaseWriterHandle`] instead.
  #[tracing::instrument(level = "info", skip_all)]
  fn run_db_healthcheck(db: &Arc<DatabaseWriter>) -> napi::Result<()> {
    let run_healthcheck = || -> anyhow::Result<()> {
      let txn = db.read_txn()?;
      let value = db
        .get(&txn, "current_session_version")?
        .ok_or(anyhow!("Missing 'current_session_version' key in LMDB"))?;
      let value = str::from_utf8(&value)?;
      tracing::info!("current_session_version: {:?}", value);
      Ok(())
    };

    if let Err(err) = run_healthcheck() {
      tracing::warn!("LMDB healthcheck failed: {:?}", err);
    }

    Ok(())
  }
}
