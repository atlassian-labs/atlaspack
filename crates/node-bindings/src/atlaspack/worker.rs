use std::sync::Arc;

use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack_napi_helpers::JsTransferable;
use napi::{Env, JsObject};
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

  ## Handle Leak Fix (ATLASPACK-V3)

  Each NodejsWorker contains 3 JsCallable instances, which wrap ThreadsafeFunctions.
  ThreadsafeFunctions by default are "ref'd", meaning they keep the Node.js event
  loop alive. This causes problems:

  1. When tests run, the worker pool is created
  2. Each worker creates 3 ThreadsafeFunctions (loadPlugin, runResolverResolve,
     runTransformerTransform)
  3. These TSFNs keep the event loop alive even after worker.terminate() is called
  4. Mocha can't exit naturally between tests, causing hangs

  The fix is to call `unref()` on each JsCallable's ThreadsafeFunction immediately
  after creation. See NodejsWorker::new() for implementation details.
*/

/// Called on the worker thread to create a reference to the NodeJs worker.
///
/// This function is called from each worker thread during initialization.
/// It creates a NodejsWorker struct that wraps the JavaScript worker's methods
/// (loadPlugin, runResolverResolve, runTransformerTransform) as ThreadsafeFunctions.
///
/// # Handle Management
///
/// The Env parameter is required to properly unref the ThreadsafeFunctions
/// created for this worker. Without unref, these TSFNs would keep the Node.js
/// event loop alive, causing test hangs and preventing clean process exit.
///
/// # Returns
///
/// A JsTransferable containing an Arc<NodejsWorker>. The JsTransferable wrapper
/// allows the worker reference to be passed back to the main thread via postMessage.
#[napi]
pub fn new_nodejs_worker(
  env: Env,
  worker: JsObject,
) -> napi::Result<JsTransferable<Arc<NodejsWorker>>> {
  // Pass the env to NodejsWorker::new so it can unref its ThreadsafeFunctions.
  // This is critical to prevent handle leaks - without unref, the TSFNs would
  // keep the Node.js event loop alive even after worker.terminate() is called.
  let worker = NodejsWorker::new(&env, worker)?;
  Ok(JsTransferable::new(Arc::new(worker)))
}

/// Clear the JsTransferable registry.
///
/// The JsTransferable registry is a global HashMap that stores Rust values
/// (wrapped in Arc) that have been passed to JavaScript via postMessage.
/// These entries persist for the lifetime of the process unless explicitly
/// cleared.
///
/// In the V3 worker pool, each worker creates a JsTransferable entry when
/// it's initialized. If the worker pool is shut down without clearing these
/// entries, they accumulate, leading to memory leaks.
///
/// This function should be called during worker pool shutdown to clean up
/// orphaned registry entries.
#[napi]
pub fn clear_transferable_registry() {
  atlaspack_napi_helpers::clear_transferable_registry();
}
