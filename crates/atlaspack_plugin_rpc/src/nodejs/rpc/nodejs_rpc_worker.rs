use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{Env, JsObject};

/// NodejsWorker is the connection to a single JavaScript worker thread.
///
/// ## Handle Management
///
/// Each NodejsWorker holds 3 JsCallable instances, and each JsCallable wraps a
/// ThreadsafeFunction (TSFN). By default, TSFNs are "ref'd", meaning they keep
/// the Node.js event loop alive. This is problematic because:
///
/// 1. Worker threads are managed by the NapiWorkerPool on the JS side
/// 2. The actual worker threads (MessagePorts) should be the only things
///    keeping the event loop alive
/// 3. When we call worker.terminate() on the JS side, the worker thread dies,
///    but the Rust-side TSFNs still hold references
/// 4. These orphaned TSFN references prevent Node.js from exiting cleanly
///
/// The fix is to call `unref()` on each JsCallable immediately after creation.
/// This allows the worker threads to be properly terminated without the TSFNs
/// keeping the event loop alive.
pub struct NodejsWorker {
  pub load_plugin_fn: JsCallable,
  pub run_resolver_resolve_fn: JsCallable,
  pub transformer_register_fn: JsCallable,
}

impl NodejsWorker {
  /// Create a new NodejsWorker from a JavaScript worker delegate object.
  ///
  /// # Arguments
  ///
  /// * `env` - The N-API environment, needed to unref the ThreadsafeFunctions
  /// * `delegate` - The JavaScript worker object with loadPlugin, runResolverResolve,
  ///   and runTransformerTransform methods
  ///
  /// # Handle Management
  ///
  /// This function creates 3 JsCallables (ThreadsafeFunctions) and immediately
  /// unrefs them. This is critical to prevent handle leaks - without unref,
  /// these TSFNs would keep the Node.js event loop alive even after the worker
  /// thread is terminated.
  ///
  /// The TSFNs can still be called after being unref'd; unref only affects
  /// whether they keep the event loop alive, not their functionality.
  pub fn new(env: &Env, delegate: JsObject) -> napi::Result<Self> {
    // Create JsCallables for each worker method
    let mut load_plugin_fn = JsCallable::new_method_bound("loadPlugin", &delegate)?;
    let mut run_resolver_resolve_fn =
      JsCallable::new_method_bound("runResolverResolve", &delegate)?;
    let mut transformer_register_fn =
      JsCallable::new_method_bound("runTransformerTransform", &delegate)?;

    // Unref all JsCallables immediately after creation.
    //
    // Without this, each JsCallable's ThreadsafeFunction would keep the Node.js
    // event loop alive. When the worker pool is shut down (worker.terminate()
    // is called), the worker threads die but these TSFNs still hold references,
    // causing Node.js to hang instead of exiting cleanly.
    //
    // By calling unref(), we allow the event loop to exit when only these
    // TSFNs are keeping it alive. The actual worker threads (MessagePorts)
    // are still the source of truth for keeping workers alive.
    load_plugin_fn.unref(env)?;
    run_resolver_resolve_fn.unref(env)?;
    transformer_register_fn.unref(env)?;

    Ok(Self {
      load_plugin_fn,
      run_resolver_resolve_fn,
      transformer_register_fn,
    })
  }
}
