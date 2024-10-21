use std::rc::Rc;

use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_napi_helpers::JsObjectExt;
use napi::CallContext;
use napi::Env;
use napi::JsObject;

/// Mappings for ./packages/core/core/src/public/Config.js#29
pub struct PublicConfig {
  config_loader: ConfigLoaderRef,
}

impl PublicConfig {
  pub fn new(env: &Env, config_loader: ConfigLoaderRef) -> napi::Result<JsObject> {
    let this = Rc::new(Self { config_loader });
    let mut obj = env.create_object()?;

    obj.define_method(env, "getConfigFrom", {
      let this = this.clone();
      move |ctx| this.get_config_from(ctx)
    })?;

    obj.define_method(env, "getConfig", {
      let this = this.clone();
      move |ctx| this.get_config(ctx)
    })?;

    Ok(obj)
  }

  fn get_config_from(&self, _ctx: CallContext<'_>) -> napi::Result<()> {
    println!("{:?}", self.config_loader.project_root);
    Ok(())
  }

  fn get_config(&self, _ctx: CallContext<'_>) -> napi::Result<()> {
    Ok(())
  }
}
