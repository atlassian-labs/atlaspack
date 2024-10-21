use atlaspack_napi_helpers::JsObjectExt;
use napi::CallContext;
use napi::Env;
use napi::JsObject;

/// Mappings for packages/core/types-internal/src/index.js#1100
pub struct Resolve;

impl Resolve {
  pub fn new(env: &Env) -> napi::Result<JsObject> {
    let mut obj = env.create_object()?;

    obj.define_method(env, "resolve", Self::resolve)?;

    Ok(obj)
  }

  fn resolve(_ctx: CallContext<'_>) -> napi::Result<()> {
    Ok(())
  }
}
