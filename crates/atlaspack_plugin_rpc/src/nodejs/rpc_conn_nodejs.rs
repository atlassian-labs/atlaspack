use atlaspack_napi_helpers::anyhow_from_napi;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{JsObject, JsUnknown};

use crate::{RpcTransformerOpts, RpcTransformerResult, RpcWorker};

/// RpcConnectionNodejs wraps the communication with a
/// single Nodejs worker thread
pub struct NodejsWorker {
  ping_fn: JsCallable,
  transformer_register_fn: JsCallable,
}

impl NodejsWorker {
  pub fn new(delegate: JsObject) -> napi::Result<Self> {
    Ok(Self {
      ping_fn: JsCallable::new_from_object_prop_bound("ping", &delegate)?,
      transformer_register_fn: JsCallable::new_from_object_prop_bound("runTransformer", &delegate)?,
    })
  }
}

impl RpcWorker for NodejsWorker {
  fn ping(&self) -> anyhow::Result<()> {
    self
      .ping_fn
      .call_with_return(
        |_env| Ok(Vec::<JsUnknown>::new()),
        |_env, _| Ok(Vec::<()>::new()),
      )
      .map_err(anyhow_from_napi)?;
    Ok(())
  }

  fn run_transformer(&self, opts: RpcTransformerOpts) -> anyhow::Result<RpcTransformerResult> {
    self
      .transformer_register_fn
      .call_with_return_serde(opts)
      .map_err(anyhow_from_napi)
  }
}
