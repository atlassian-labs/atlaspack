use std::path::PathBuf;

use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_napi_helpers::anyhow_from_napi;
use atlaspack_napi_helpers::anyhow_to_napi;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{bindgen_prelude::FromNapiValue, JsString};
use napi::{Env, JsObject};
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

  pub fn load_plugin<T: Serialize + Send + Sync + 'static>(
    &self,
    opts: LoadPluginOptions<T>,
    config_loader: ConfigLoaderRef,
  ) -> anyhow::Result<()> {
    self
      .load_plugin_fn
      .call_with_return(
        move |env| {
          let obj = env.to_js_value(&opts)?;
          let mut obj = JsObject::from_unknown(obj)?;

          let napi_config_loader = NapiConfigLoader::bind(config_loader, env)?;
          obj.set_named_property("configLoader", napi_config_loader)?;

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
pub struct LoadPluginOptions<T: Serialize + Send + Sync + 'static> {
  pub kind: LoadPluginKind,
  pub specifier: String,
  pub resolve_from: PathBuf,
  pub data: T,
}

struct NapiConfigLoader;

impl NapiConfigLoader {
  fn bind(config_loader: ConfigLoaderRef, env: &Env) -> napi::Result<JsObject> {
    let mut napi_config_loader = env.create_object()?;

    napi_config_loader.set_named_property(
      "loadJsonConfig",
      env.create_function_from_closure("loadJsonConfig", {
        let config_loader = config_loader.clone();
        move |ctx| {
          let file_path_js = ctx.get::<JsString>(0)?;
          let file_path = file_path_js.into_utf8()?.into_owned()?;

          let result = config_loader
            .load_json_config::<serde_json::Value>(&file_path)
            .map_err(anyhow_to_napi)?;

          let result_js = ctx.env.to_js_value(&result)?;
          Ok(result_js)
        }
      })?,
    )?;

    napi_config_loader.set_named_property(
      "loadJsonConfigFrom",
      env.create_function_from_closure("loadJsonConfigFrom", {
        let config_loader = config_loader.clone();
        move |ctx| {
          let search_path_js = ctx.get::<JsString>(0)?;
          let search_path = search_path_js.into_utf8()?.into_owned()?;

          let file_path_js = ctx.get::<JsString>(1)?;
          let file_path = file_path_js.into_utf8()?.into_owned()?;

          let result = config_loader
            .load_json_config_from::<serde_json::Value>(&PathBuf::from(search_path), &file_path)
            .map_err(anyhow_to_napi)?;

          let result_js = ctx.env.to_js_value(&result)?;
          Ok(result_js)
        }
      })?,
    )?;

    Ok(napi_config_loader)
  }
}
