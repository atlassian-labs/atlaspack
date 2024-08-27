use std::path::Path;

use anyhow::Context;
use atlaspack_napi_helpers::anyhow_from_napi;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{JsObject, JsUnknown};

use crate::RpcWorker;

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
      transformer_register_fn: JsCallable::new_from_object_prop_bound(
        "transformerRegister",
        &delegate,
      )?,
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

  fn transformer_register(&self, resolve_from: &Path, specifier: &str) -> anyhow::Result<()> {
    self
      .transformer_register_fn
      .call_with_return(
        {
          let specifier = specifier.to_string();
          let resolve_from = resolve_from.to_str().context("")?.to_string();
          move |env| {
            Ok(vec![
              env.create_string(&resolve_from)?.into_unknown(),
              env.create_string(&specifier)?.into_unknown(),
            ])
          }
        },
        |_env, _| Ok(Vec::<()>::new()),
      )
      .map_err(anyhow_from_napi)?;
    Ok(())
  }
}
