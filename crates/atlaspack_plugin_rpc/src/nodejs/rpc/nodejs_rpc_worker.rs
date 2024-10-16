use std::path::PathBuf;

use atlaspack_napi_helpers::anyhow_from_napi;
use atlaspack_napi_helpers::js_callable::map_return_serde;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::bindgen_prelude::FromNapiValue;
use napi::JsObject;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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
      transformer_register_fn: bind("runTransformerTransform")?,
    })
  }

  pub fn load_plugin<T: Serialize + Send + Sync + 'static>(
    &self,
    opts: LoadPluginOptions,
    data: Option<T>,
  ) -> anyhow::Result<()> {
    self
      .load_plugin_fn
      .call_with_return(
        move |env| {
          let obj = env.to_js_value(&opts)?;
          let mut obj = JsObject::from_unknown(obj)?;
          if let Some(data) = data {
            obj.set_named_property("data", env.to_js_value(&data)?)?;
          }
          Ok(vec![obj.into_unknown()])
        },
        |_env, _value| Ok(()),
      )
      .map_err(anyhow_from_napi)
  }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadPluginKind {
  Resolver,
  Transformer,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
}
