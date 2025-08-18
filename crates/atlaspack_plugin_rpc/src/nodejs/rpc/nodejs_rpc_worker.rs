use async_trait::async_trait;
use atlaspack_core::{plugin::Resolved, types::Code};
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::{bindgen_prelude::FromNapiValue, JsBuffer, JsObject, JsString, JsUnknown};

use crate::javascript_plugin_api::{
  JavaScriptPluginAPI, LoadPluginOptions, RpcAssetResult, RpcTransformerOpts, RunResolverResolve,
};

/// NodejsWorker is the connection to a single JavaScript worker thread
pub struct NodejsWorker {
  load_plugin_fn: JsCallable,
  run_resolver_resolve_fn: JsCallable,
  transformer_register_fn: JsCallable,
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
}

#[async_trait]
impl JavaScriptPluginAPI for NodejsWorker {
  async fn resolve(&self, opts: RunResolverResolve) -> anyhow::Result<Resolved> {
    self.run_resolver_resolve_fn.call_serde(opts).await
  }

  async fn transform(
    &self,
    code: Code,
    source_map: Option<String>,
    run_transformer_opts: RpcTransformerOpts,
  ) -> anyhow::Result<(RpcAssetResult, Vec<u8>, Option<String>)> {
    let result = self
      .transformer_register_fn
      .call(
        move |env| {
          let run_transformer_opts = env.to_js_value(&run_transformer_opts)?;

          let mut contents = env.create_buffer(code.len())?;
          contents.copy_from_slice(&code);

          let map = if let Some(map) = source_map {
            env.create_string(&map)?.into_unknown()
          } else {
            env.get_undefined()?.into_unknown()
          };

          Ok(vec![run_transformer_opts, contents.into_unknown(), map])
        },
        |env, return_value| {
          let return_value = JsObject::from_unknown(return_value)?;

          let transform_result = return_value.get_element::<JsUnknown>(0)?;
          let transform_result = env.from_js_value::<RpcAssetResult, _>(transform_result)?;

          let contents = return_value.get_element::<JsBuffer>(1)?;
          let contents = contents.into_value()?.to_vec();

          let map = return_value.get_element::<JsString>(2)?.into_utf8()?;
          let map = if map.is_empty() {
            None
          } else {
            Some(map.into_owned()?)
          };

          Ok((transform_result, contents, map))
        },
      )
      .await?;

    Ok(result)
  }

  async fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    self.load_plugin_fn.call_serde(opts).await
  }
}
