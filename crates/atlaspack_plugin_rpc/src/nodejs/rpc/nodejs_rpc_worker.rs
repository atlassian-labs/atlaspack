use std::path::PathBuf;

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
    let bind = |method_name: &str| JsCallable::new_method_bound(method_name, &delegate);

    Ok(Self {
      load_plugin_fn: bind("loadPlugin")?,
      run_resolver_resolve_fn: bind("runResolverResolve")?,
      transformer_register_fn: bind("runTransformerTransform")?,
    })
  }

  pub async fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    self.load_plugin_fn.call_serde(opts).await
  }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadPluginKind {
  Resolver,
  Transformer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
}
