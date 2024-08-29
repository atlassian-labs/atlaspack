use std::path::Path;

use anyhow::Context;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::types::Asset;
use atlaspack_napi_helpers::anyhow_from_napi;
use atlaspack_napi_helpers::js_callable;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{JsObject, JsUnknown};

use crate::RpcWorker;

/// RpcConnectionNodejs wraps the communication with a
/// single Nodejs worker thread
pub struct NodejsWorker {
  ping_fn: JsCallable,
  register_transformer_fn: JsCallable,
  transform_transformer_fn: JsCallable,
}

impl NodejsWorker {
  pub fn new(delegate: JsObject) -> napi::Result<Self> {
    Ok(Self {
      ping_fn: JsCallable::new_from_object_prop_bound("ping", &delegate)?,
      register_transformer_fn: JsCallable::new_from_object_prop_bound(
        "registerTransformer",
        &delegate,
      )?,
      transform_transformer_fn: JsCallable::new_from_object_prop_bound(
        "transformTransformer",
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

  fn register_transformer(&self, resolve_from: &Path, specifier: &str) -> anyhow::Result<()> {
    self
      .register_transformer_fn
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

  fn transform_transformer(&self, key: &str, asset: &Asset) -> anyhow::Result<TransformResult> {
    Ok(self.transform_transformer_fn.call_with_return(
      js_callable::map_params_serde((key.to_string(), asset.clone())),
      js_callable::map_return_serde::<TransformResult>(),
    )?)
  }
}
