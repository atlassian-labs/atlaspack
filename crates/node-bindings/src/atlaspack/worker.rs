use std::{
  sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
  },
  thread,
};

use atlaspack::rpc::nodejs::NodejsWorker;
use atlaspack_napi_helpers::JsTransferable;
use napi::{bindgen_prelude::FromNapiValue, Env, JsFunction, JsObject, JsUndefined, JsUnknown};
use napi_derive::napi;

/// This function is run in the Nodejs worker context upon initialization
/// to notify the main thread that a Nodejs worker thread has started
///
/// A Rust channel is transferred to the worker via JavaScript `worker.postMessage`.
/// The worker then calls `register_worker`, supplying it with an object containing
/// callbacks.
///
/// The callbacks are later called from the main thread to send work to the worker.
///
/// |-------------| --- Init channel ----> |-------------------|
/// | Main Thread |                        | Worker Thread (n) |
/// |-------------| <-- Worker wrapper --- |-------------------|
///
///                 **Later During Build**
///
///                 -- Resolver.resolve -->
///                 <- DependencyResult ---
///
///                 -- Transf.transform -->
///                 <--- Array<Asset> -----
#[napi]
pub fn register_worker(
  env: Env,
  channel: JsTransferable<Sender<NodejsWorker>>,
  worker: JsObject,
) -> napi::Result<JsUndefined> {
  let worker = NodejsWorker::new(worker)?;
  let tx_worker = channel.take()?;
  if tx_worker.send(worker).is_err() {
    return Err(napi::Error::from_reason("Unable to register worker"));
  }
  env.get_undefined()
}

// Called on the worker thread to create a reference to the NodeJs worker
#[napi]
pub fn new_nodejs_worker(worker: JsObject) -> napi::Result<JsTransferable<Arc<NodejsWorker>>> {
  let worker = NodejsWorker::new(worker)?;
  Ok(JsTransferable::new(Arc::new(worker)))
}

// Called on the main thread to get the worker references
pub fn get_workers(
  env: &Env,
  worker_pool: &JsObject,
) -> napi::Result<Receiver<Vec<Arc<NodejsWorker>>>> {
  let (tx_workers, rx_workers) = channel::<Vec<Arc<NodejsWorker>>>();

  let worker_count = worker_pool.get_named_property::<JsFunction>("workerCount")?;
  let worker_count = worker_count
    .call::<JsUnknown>(Some(worker_pool), &[])?
    .coerce_to_number()?
    .get_uint32()?;

  let (tx, rx) = channel::<Arc<NodejsWorker>>();
  thread::spawn(move || {
    let mut workers = vec![];
    // tx is dropped with Nodejs GC so we have to use
    // a static workerCount number to break the loop
    // to avoid the loop waiting for Nodejs GC
    for _ in 0..worker_count {
      let Ok(worker) = rx.recv() else {
        break;
      };
      workers.push(worker);
    }
    tx_workers.send(workers).unwrap();
  });

  // Unwrap promise and pass result to separate thread
  // to avoid deadlocking the main thread
  let get_workers = worker_pool.get_named_property::<JsFunction>("getWorkers")?;
  let workers_promise = get_workers
    .call::<JsUnknown>(Some(worker_pool), &[])?
    .coerce_to_object()?;
  let workers_promise_then = workers_promise.get_named_property::<JsFunction>("then")?;

  let workers_promise_then_fn = env.create_function_from_closure("workers_promise_then_fn", {
    move |ctx| {
      let arg0 = ctx.get::<JsObject>(0)?;
      for index in 0..worker_count {
        let current: JsUnknown = arg0.get_element(index)?;
        let nodejs_worker = JsTransferable::<Arc<NodejsWorker>>::from_unknown(current)?;
        let nodejs_worker = nodejs_worker.get()?.clone();
        tx.send(nodejs_worker).unwrap();
      }
      Ok(())
    }
  })?;

  workers_promise_then.call::<JsFunction>(Some(&workers_promise), &[workers_promise_then_fn])?;

  Ok(rx_workers)
}
