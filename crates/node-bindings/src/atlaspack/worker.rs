use std::sync::Arc;

use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack_napi_helpers::JsTransferable;
use napi::JsObject;
use napi_derive::napi;

/*
  Main Thread                       Worker Thread (n)

  Spawns worker threads      -->    Constructs native NodeJsWorker
                             <--    postMessage "ptr"
  Waits for pointers then
  forwards them to napi.
  napi unwraps pointers to
  underlying NodeJsWorker
  then calls methods on it
  during build
*/

/// Called on the worker thread to create a reference to the NodeJs worker
#[napi]
pub fn new_nodejs_worker(worker: JsObject) -> napi::Result<JsTransferable<Arc<NodejsWorker>>> {
  let worker = NodejsWorker::new(worker)?;
  Ok(JsTransferable::new(Arc::new(worker)))
}
