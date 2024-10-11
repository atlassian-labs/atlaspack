use std::path::PathBuf;

use atlaspack_napi_helpers::anyhow_from_napi;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::JsObject;
use serde::{Deserialize, Serialize};

/// NodejsWorker is the connection to a single JavaScript worker thread
pub struct NodejsWorker {
  pub load_plugin_fn: JsCallable,
  pub run_resolver_resolve_fn: JsCallable,
  pub transformer_register_fn: JsCallable,
}

impl NodejsWorker {
  pub fn new(delegate: JsObject) -> napi::Result<Self> {
    let bind = |method_name: &str| JsCallable::new_from_object_prop_bound(method_name, &delegate);

    Ok(Self {
      load_plugin_fn: bind("loadPlugin")?,
      run_resolver_resolve_fn: bind("runResolverResolve")?,
      transformer_register_fn: bind("runTransformer")?,
    })
  }

  pub fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    self
      .load_plugin_fn
      .call_with_return_serde(opts)
      .map_err(anyhow_from_napi)
  }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadPluginKind {
  Resolver,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
}
