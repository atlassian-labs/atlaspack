use core::str;
use std::sync::Arc;

use anyhow::anyhow;
use lmdb_js_lite::writer::DatabaseWriter;
use lmdb_js_lite::LMDB;
use napi::bindgen_prelude::External;
use napi::bindgen_prelude::FromNapiValue;
use napi::Env;
use napi::JsObject;
use napi::JsUnknown;
use napi_derive::napi;

use atlaspack::file_system::FileSystemRef;
use atlaspack::rpc::nodejs::NodejsRpcFactory;
use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack::Atlaspack;
use atlaspack::AtlaspackInitOptions;
use atlaspack::WatchEvents;
use atlaspack_napi_helpers::js_callable::JsCallable;
use atlaspack_napi_helpers::napi_threads;
use atlaspack_napi_helpers::JsTransferable;
use atlaspack_napi_helpers::TupleResult;
use atlaspack_package_manager::PackageManagerRef;
use parking_lot::Mutex;

use super::file_system_napi::FileSystemNapi;
use super::package_manager_napi::PackageManagerNapi;
use super::serialize_asset_graph::serialize_asset_graph;

#[napi(object)]
pub struct AtlaspackNapiOptions {
  pub fs: Option<JsObject>,
  pub options: JsObject,
  pub package_manager: Option<JsObject>,
  pub threads: Option<u32>,
  pub napi_worker_pool: JsObject,
}

pub type AtlaspackNapi = External<Arc<Mutex<Atlaspack>>>;

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_create(
  env: Env,
  napi_options: AtlaspackNapiOptions,
  lmdb: &LMDB,
) -> napi::Result<JsObject> {
  let thread_id = std::thread::current().id();
  tracing::trace!(?thread_id, "atlaspack-napi initialize");

  // Wrap the JavaScript-supplied FileSystem
  let fs: Option<FileSystemRef> = if let Some(fs) = napi_options.fs {
    Some(Arc::new(FileSystemNapi::new(&env, &fs)?))
  } else {
    None
  };

  // Wrap the JavaScript supplied PackageManager
  let package_manager: Option<PackageManagerRef> = if let Some(pm) = napi_options.package_manager {
    Some(Arc::new(PackageManagerNapi::new(&env, &pm)?))
  } else {
    None
  };

  // Get access to LMDB reference
  let db_handle = lmdb.get_database_napi()?.clone();
  let db_writer = db_handle.database();
  let db = db_writer.clone();
  run_db_health_check(db_writer)?;

  // Get Atlaspack Options
  let options = env.from_js_value(napi_options.options)?;
  let get_workers = JsCallable::new_method_bound("getWorkers", &napi_options.napi_worker_pool)?;

  napi_threads::spawn(
    &env,
    // Separate thread
    move || {
      let workers = get_workers.call_blocking(
        |_env| Ok(vec![]),
        |_env, workers| {
          let workers_arr = workers.coerce_to_object()?;
          let mut workers = vec![];
          for i in 0..workers_arr.get_array_length()? {
            let worker = workers_arr.get_element::<JsUnknown>(i)?;
            let worker = JsTransferable::<Arc<NodejsWorker>>::from_unknown(worker)?;
            workers.push(worker.get()?.clone());
          }
          Ok(workers)
        },
      )?;

      let rpc = Arc::new(NodejsRpcFactory::new(workers)?);
      let atlaspack = Atlaspack::new(AtlaspackInitOptions {
        db,
        fs,
        options,
        package_manager,
        rpc,
      });

      Ok(External::new(Arc::new(Mutex::new(atlaspack))))
    },
    // Nodejs Thread
    napi_threads::map_to_ok_tuple,
    napi_threads::map_to_err_tuple,
  )
}

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_build_asset_graph(
  env: Env,
  atlaspack: AtlaspackNapi,
) -> napi::Result<JsObject> {
  napi_threads::spawn(
    &env,
    // Separate thread
    move || atlaspack.lock().build_asset_graph(),
    // Nodejs Thread
    |env, asset_graph| TupleResult::ok(&env, serialize_asset_graph(&env, &asset_graph)?),
    napi_threads::map_to_err_tuple,
  )
}

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_respond_to_fs_events(
  env: Env,
  atlaspack: AtlaspackNapi,
  options: JsObject,
) -> napi::Result<JsObject> {
  let options = env.from_js_value::<WatchEvents, _>(options)?;

  napi_threads::spawn(
    &env,
    // Separate thread
    move || atlaspack.lock().respond_to_fs_events(options),
    // Nodejs Thread
    napi_threads::map_to_ok_tuple,
    napi_threads::map_to_err_tuple,
  )
}

/// Check that the LMDB database is healthy
///
/// JavaScript does all its writes through a single thread, which is not this handle. If we want
/// to sequence writes with the JavaScript writes, we should be using the
/// [`lmdb_js_lite::writer::DatabaseWriterHandle`] instead.
#[tracing::instrument(level = "info", skip_all)]
fn run_db_health_check(db: &Arc<DatabaseWriter>) -> napi::Result<()> {
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
