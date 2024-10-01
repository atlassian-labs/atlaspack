use atlaspack_core::plugin::Resolved;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{JsObject, JsUnknown};

use crate::{
  LoadPluginOptions, RpcTransformerOpts, RpcTransformerResult, RpcWorker, RunResolverResolve,
};

/// RpcConnectionNodejs wraps the communication with a
/// single Nodejs worker thread
pub struct NodejsWorker {
  ping_fn: JsCallable,
  load_plugin_fn: JsCallable,
  run_resolver_resolve_fn: JsCallable,
  transformer_register_fn: JsCallable,
}

impl NodejsWorker {
  pub fn new(delegate: JsObject) -> napi::Result<Self> {
    Ok(Self {
      ping_fn: JsCallable::new_from_object_prop_bound("ping", &delegate)?,
      load_plugin_fn: JsCallable::new_from_object_prop_bound("loadPlugin", &delegate)?,
      run_resolver_resolve_fn: JsCallable::new_from_object_prop_bound(
        "runResolverResolve",
        &delegate,
      )?,
      transformer_register_fn: JsCallable::new_from_object_prop_bound("runTransformer", &delegate)?,
    })
  }
}

impl RpcWorker for NodejsWorker {
  fn ping(&self) -> anyhow::Result<()> {
    self.ping_fn.call(
      |_env| Ok(Vec::<JsUnknown>::new()),
      |_env, _| Ok(Vec::<()>::new()),
    )?;
    Ok(())
  }

  fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    self.load_plugin_fn.call_serde(opts)
  }

  fn run_resolver_resolve(&self, opts: RunResolverResolve) -> anyhow::Result<Resolved> {
    self.run_resolver_resolve_fn.call_serde(opts)
  }

  fn run_transformer(&self, opts: RpcTransformerOpts) -> anyhow::Result<RpcTransformerResult> {
    self.transformer_register_fn.call_serde(opts)
  }
}
