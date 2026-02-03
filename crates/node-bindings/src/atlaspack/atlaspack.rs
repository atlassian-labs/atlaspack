use core::str;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

use anyhow::anyhow;
use atlaspack::Atlaspack;
use atlaspack::AtlaspackError;
use atlaspack::AtlaspackInitOptions;
use atlaspack::WatchEvents;
use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack_core::bundle_graph::bundle_graph_from_js::BundleGraphFromJs;
use atlaspack_core::types::Environment;
use atlaspack_napi_helpers::JsTransferable;
use atlaspack_napi_helpers::js_callable::JsCallable;
use lmdb_js_lite::DatabaseHandle;
use lmdb_js_lite::LMDBJsLite;
use napi::Env;
use napi::JsObject;
use napi::JsUnknown;
use napi::bindgen_prelude::External;
use napi::bindgen_prelude::FromNapiValue;
use napi_derive::napi;

use atlaspack::file_system::FileSystemRef;
use atlaspack::rpc::nodejs::NodejsRpcFactory;
use atlaspack_package_manager::PackageManagerRef;
use parking_lot::RwLock;

use crate::atlaspack::package_result_napi::JsPackageResult;

use super::file_system_napi::FileSystemNapi;
use super::napi_result::NapiAtlaspackResult;
use super::package_manager_napi::PackageManagerNapi;
use super::serialize_asset_graph::serialize_asset_graph;
use super::serialize_bundle_graph::serialize_bundle_graph;

#[napi(object)]
pub struct AtlaspackNapiOptions {
  pub fs: Option<JsObject>,
  pub options: JsObject,
  pub package_manager: Option<JsObject>,
  pub threads: Option<u32>,
  pub napi_worker_pool: JsObject,
}

pub type AtlaspackNapi = External<Arc<RwLock<Atlaspack>>>;

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_create(
  env: Env,
  napi_options: AtlaspackNapiOptions,
  lmdb: &LMDBJsLite,
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
  let db_handle = lmdb.get_database().clone();
  atlaspack_napi_run_db_health_check(&db_handle)?;

  // Get Atlaspack Options
  let options = env.from_js_value(napi_options.options)?;
  let get_workers = JsCallable::new_method_bound("getWorkers", &napi_options.napi_worker_pool)?;

  let (deferred, promise) = env.create_deferred()?;
  thread::spawn({
    let db = db_handle.clone();
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
      tracing::trace!(?thread_id, "atlaspack-napi resolve");
      deferred.resolve(move |env| match atlaspack {
        Ok(atlaspack) => {
          NapiAtlaspackResult::ok(&env, External::new(Arc::new(RwLock::new(atlaspack))))
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

fn resolve_commit_ok(env: Env) -> napi::Result<JsObject> {
  NapiAtlaspackResult::ok(&env, ())
}

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_build_asset_graph(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;
  let (second_deferred, second_promise) = env.create_deferred()?;

  let mut js_result = env.create_object()?;
  js_result.set_named_property("assetGraphPromise", promise)?;
  js_result.set_named_property("commitPromise", second_promise)?;

  thread::spawn({
    let atlaspack_ref = atlaspack_napi.clone();
    move || {
      let result = {
        let atlaspack = atlaspack_ref.write();
        atlaspack.build_asset_graph()
      };

      // "deferred.resolve" closure executes on the JavaScript thread.
      // Errors are returned as a resolved value because they need to be serialized and are
      // not supplied as JavaScript Error types. The JavaScript layer needs to handle conversions
      let mut commit_deferred_opt = Some(second_deferred);
      deferred.resolve(move |env| {
        match result {
          Ok((asset_graph, had_previous_graph)) => {
            let serialize_result =
              serialize_asset_graph(&env, &asset_graph.clone(), had_previous_graph)?;
            if let Some(commit_deferred) = commit_deferred_opt.take() {
              thread::spawn(move || {
                {
                  let atlaspack = atlaspack_ref.write();
                  atlaspack.commit_assets(&asset_graph).unwrap();
                }
                commit_deferred.resolve(resolve_commit_ok)
              });
            }

            NapiAtlaspackResult::ok(&env, serialize_result)
          }
          Err(error) => {
            // Resolve the commit promise immediately since there's nothing to commit on error
            if let Some(commit_deferred) = commit_deferred_opt.take() {
              commit_deferred.resolve(resolve_commit_ok);
            }
            let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
            NapiAtlaspackResult::error(&env, js_object)
          }
        }
      })
    }
  });

  Ok(js_result)
}

#[tracing::instrument(level = "debug", skip_all)]
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
      let atlaspack = atlaspack.write();
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

#[tracing::instrument(level = "info", skip_all)]
#[napi]
pub fn atlaspack_napi_build_bundle_graph(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;
  let (second_deferred, second_promise) = env.create_deferred()?;

  let mut js_result = env.create_object()?;
  js_result.set_named_property("bundleGraphPromise", promise)?;
  js_result.set_named_property("commitPromise", second_promise)?;

  thread::spawn({
    let atlaspack_ref = atlaspack_napi.clone();
    move || {
      let result = {
        let atlaspack = atlaspack_ref.write();
        atlaspack.build_bundle_graph()
      };

      let mut commit_deferred_opt = Some(second_deferred);
      deferred.resolve(move |env| match result {
        Ok((asset_graph, bundle_graph_delta, had_previous_graph)) => {
          let serialize_result =
            serialize_bundle_graph(&env, &bundle_graph_delta.bundle_graph, had_previous_graph)?;

          if let Some(commit_deferred) = commit_deferred_opt.take() {
            thread::spawn(move || {
              {
                let atlaspack = atlaspack_ref.write();
                atlaspack.commit_assets(&asset_graph).unwrap();
              }
              commit_deferred.resolve(resolve_commit_ok)
            });
          }

          NapiAtlaspackResult::ok(&env, serialize_result)
        }
        Err(error) => {
          if let Some(commit_deferred) = commit_deferred_opt.take() {
            commit_deferred.resolve(resolve_commit_ok);
          }
          let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
          NapiAtlaspackResult::error(&env, js_object)
        }
      })
    }
  });

  Ok(js_result)
}

#[napi]
pub fn atlaspack_napi_load_bundle_graph(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
  nodes_json: String,
  edges: Vec<(u32, u32, u8)>,
  public_id_by_asset_id: HashMap<String, String>,
  environments_json: String,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;

  // Move all parsing and deserialization off the JS thread
  thread::spawn({
    let atlaspack = atlaspack_napi.clone();
    move || {
      let result: anyhow::Result<()> = (|| {
        let environments: Vec<Environment> = serde_json::from_str(&environments_json)
          .map_err(|e| anyhow::anyhow!("Failed to parse environments JSON: {}", e))?;

        let nodes = BundleGraphFromJs::deserialize_from_json(nodes_json, &environments)?;

        let atlaspack = atlaspack.write();
        atlaspack.load_bundle_graph(
          nodes,
          edges
            .into_iter()
            .map(|(from, to, edge_type)| (from, to, edge_type.into()))
            .collect(),
          public_id_by_asset_id,
          environments,
        )
      })();

      deferred.resolve(move |env| match result {
        Ok(()) => NapiAtlaspackResult::ok(&env, ()),
        Err(error) => {
          let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
          NapiAtlaspackResult::error(&env, js_object)
        }
      })
    }
  });
  Ok(promise)
}

#[napi]
pub fn atlaspack_napi_package(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
  bundle_id: String,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;
  thread::spawn({
    let atlaspack = atlaspack_napi.clone();
    move || {
      let atlaspack = atlaspack.read();
      let result = atlaspack.package(bundle_id);
      deferred.resolve(move |env| match result {
        Ok(result) => NapiAtlaspackResult::ok(&env, JsPackageResult::from(result)),
        Err(error) => {
          let js_object = env.to_js_value(&AtlaspackError::from(&error))?;
          NapiAtlaspackResult::error(&env, js_object)
        }
      })
    }
  });
  Ok(promise)
}

#[tracing::instrument(level = "debug", skip_all)]
#[napi]
pub fn atlaspack_napi_complete_session(
  env: Env,
  atlaspack_napi: AtlaspackNapi,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;

  thread::spawn({
    let atlaspack_ref = atlaspack_napi.clone();
    move || {
      let stats = {
        let atlaspack = atlaspack_ref.write();
        // Use tokio runtime to await the async function
        atlaspack
          .runtime
          .block_on(atlaspack.complete_cache_session())
      };

      deferred.resolve(move |env| Ok(env.to_js_value(&stats)))
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
pub fn atlaspack_napi_run_db_health_check(db: &DatabaseHandle) -> napi::Result<()> {
  let run_healthcheck = || -> anyhow::Result<()> {
    let txn = db.database().read_txn()?;
    let value = db
      .database()
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
