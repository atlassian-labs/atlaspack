use core::str;
use std::sync::Arc;
use std::thread;

use anyhow::anyhow;
use atlaspack::AtlaspackError;
use atlaspack::AtlaspackInitOptions;
use atlaspack::WatchEvents;
use lmdb_js_lite::writer::DatabaseWriter;
use lmdb_js_lite::LMDB;
use napi::Env;
use napi::JsObject;
use napi_derive::napi;

use atlaspack::file_system::FileSystemRef;
use atlaspack::rpc::nodejs::NodejsRpcFactory;
use atlaspack_package_manager::PackageManagerRef;

use crate::atlaspack::worker::get_workers;

use super::atlaspack_lazy::AtlaspackLazy;
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

#[napi]
pub struct AtlaspackNapi {
  atlaspack: AtlaspackLazy,
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

    let rx_workers = get_workers(&env, &napi_options.napi_worker_pool)?;
    let rpc = Arc::new(NodejsRpcFactory::new(rx_workers)?);
    let options = env.from_js_value(napi_options.options)?;

    let atlaspack = AtlaspackLazy::new(AtlaspackInitOptions {
      db,
      fs,
      options,
      package_manager,
      rpc,
    });

    Ok(Self { atlaspack })
  }

  #[tracing::instrument(level = "info", skip_all)]
  #[napi]
  pub fn build_asset_graph(&self, env: Env) -> napi::Result<JsObject> {
    let (deferred, promise) = env.create_deferred()?;

    thread::spawn({
      let atlaspack = self.atlaspack.clone();
      move || {
        let atlaspack = match atlaspack.get() {
          Ok(atlaspack) => atlaspack,
          Err(error) => return deferred.reject(napi::Error::from_reason(format!("{:?}", error))),
        };

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
  pub fn respond_to_fs_events(&self, env: Env, options: JsObject) -> napi::Result<JsObject> {
    let (deferred, promise) = env.create_deferred()?;
    let options = env.from_js_value::<WatchEvents, _>(options)?;

    thread::spawn({
      let atlaspack = self.atlaspack.clone();
      move || {
        let atlaspack = match atlaspack.get() {
          Ok(atlaspack) => atlaspack,
          Err(error) => return deferred.reject(napi::Error::from_reason(format!("{:?}", error))),
        };

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
