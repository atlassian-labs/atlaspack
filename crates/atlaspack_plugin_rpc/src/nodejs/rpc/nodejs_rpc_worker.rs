use std::{path::PathBuf, sync::Arc};

use atlaspack_core::static_resolver::StaticResolver;
use atlaspack_napi_helpers::js_callable::JsCallable;
use atlaspack_napi_helpers::{anyhow_from_napi, anyhow_to_napi};
use napi::{bindgen_prelude::FromNapiValue, JsObject, JsString};
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
      transformer_register_fn: bind("runTransformerTransform")?,
    })
  }

  pub fn load_plugin(
    &self,
    opts: LoadPluginOptions,
    static_resolver: Arc<StaticResolver>,
  ) -> anyhow::Result<()> {
    self
      .load_plugin_fn
      .call_with_return(
        move |env| {
          let opts = env.to_js_value(&opts)?;
          let mut opts = JsObject::from_unknown(opts)?;

          let resolve_fn = env.create_function_from_closure("resolve", move |ctx| {
            let from_js = ctx.get::<JsString>(0)?;
            let from = from_js.into_utf8()?.into_owned()?;

            let to_js = ctx.get::<JsString>(1)?;
            let to = to_js.into_utf8()?.into_owned()?;
            let to_path = PathBuf::from(to);

            let result = static_resolver
              .resolve(&from, &to_path)
              .map_err(anyhow_to_napi)?;

            ctx.env.to_js_value(&result)
          })?;

          opts.set_named_property("resolve", resolve_fn)?;

          Ok(vec![opts.into_unknown()])
        },
        |_env, _ret| Ok(()),
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginOptions {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
}
