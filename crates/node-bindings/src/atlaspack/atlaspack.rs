use core::str;
use std::sync::Arc;
use std::thread;

use anyhow::anyhow;
use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack::Atlaspack;
use atlaspack::AtlaspackError;
use atlaspack::AtlaspackInitOptions;
use atlaspack::WatchEvents;
use atlaspack_napi_helpers::js_callable::JsCallable;
use atlaspack_napi_helpers::JsTransferable;
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
use atlaspack_package_manager::PackageManagerRef;
use parking_lot::Mutex;

use super::file_system_napi::FileSystemNapi;
use super::napi_result::NapiAtlaspackResult;
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

  let (deferred, promise) = env.create_deferred()?;

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
  atlaspack_napi_run_db_health_check(db_writer)?;

  // Get Atlaspack Options
  let options = env.from_js_value(napi_options.options)?;
  let get_workers = JsCallable::new_method_bound("getWorkers", &napi_options.napi_worker_pool)?;

  thread::spawn({
    let db = db_writer.clone();
    move || {
      let workers = get_workers
        .call_blocking(
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
        )
        .unwrap();

      let rpc = Arc::new(NodejsRpcFactory::new(workers).unwrap());
      let atlaspack = Atlaspack::new(AtlaspackInitOptions {
        db,
        fs,
        options,
        package_manager,
        rpc,
      });

      deferred.resolve(move |env| match atlaspack {
        Ok(atlaspack) => {
          NapiAtlaspackResult::ok(&env, External::new(Arc::new(Mutex::new(atlaspack))))
        }
        Err(error) => {
          let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
          NapiAtlaspackResult::error(&env, js_object)
        }
      })
    }
  });

  Ok(promise)
}

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_build_asset_graph(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;

  thread::spawn({
    let atlaspack = atlaspack_napi.clone();
    move || {
      let atlaspack = atlaspack.lock();
      let result = atlaspack.build_asset_graph();

      // "deferred.resolve" closure executes on the JavaScript thread.
      // Errors are returned as a resolved value because they need to be serialized and are
      // not supplied as JavaScript Error types. The JavaScript layer needs to handle conversions
      deferred.resolve(move |env| match result {
        Ok(asset_graph) => {
          NapiAtlaspackResult::ok(&env, serialize_asset_graph(&env, &asset_graph)?)
        }
        Err(error) => {
          let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
          NapiAtlaspackResult::error(&env, js_object)
        }
      })
    }
  });

  Ok(promise)
}

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_respond_to_fs_events(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
  options: JsObject,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;
  let options = env.from_js_value::<WatchEvents, _>(options)?;

  thread::spawn({
    let atlaspack = atlaspack_napi.clone();
    move || {
      let atlaspack = atlaspack.lock();
      let result = atlaspack.respond_to_fs_events(options);

      deferred.resolve(move |env| match result {
        Ok(should_rebuild) => NapiAtlaspackResult::ok(&env, should_rebuild),
        Err(error) => {
          let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
          NapiAtlaspackResult::error(&env, js_object)
        }
      })
    }
  });

  Ok(promise)
}

/// Check that the LMDB database is healthy
///
/// JavaScript does all its writes through a single thread, which is not this handle. If we want
/// to sequence writes with the JavaScript writes, we should be using the
/// [`lmdb_js_lite::writer::DatabaseWriterHandle`] instead.
#[tracing::instrument(level = "info", skip_all)]
pub fn atlaspack_napi_run_db_health_check(db: &Arc<DatabaseWriter>) -> napi::Result<()> {
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
