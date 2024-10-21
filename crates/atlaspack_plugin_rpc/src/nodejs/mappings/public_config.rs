use atlaspack_napi_helpers::JsObjectExt;
use napi::CallContext;
use napi::Env;
use napi::JsObject;

/// Mappings for ./packages/core/core/src/public/Config.js#29
pub struct PublicConfig;

impl PublicConfig {
  pub fn new(env: &Env) -> napi::Result<JsObject> {
    let mut obj = env.create_object()?;

    obj.define_method(env, "getConfigFrom", Self::get_config_from)?;

    Ok(obj)
  }

  fn get_config_from(ctx: CallContext<'_>) -> napi::Result<()> {
    Ok(())
  }
}
