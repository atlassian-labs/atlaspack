use napi::bindgen_prelude::ToNapiValue;
use napi::CallContext;
use napi::Env;
use napi::JsObject;

pub trait JsObjectExt {
  /// Attach a method to a JavaScript object
  fn define_method<R, F>(&mut self, env: &Env, method_name: &str, method: F) -> napi::Result<()>
  where
    F: 'static + Fn(CallContext<'_>) -> napi::Result<R>,
    R: ToNapiValue;
}

impl JsObjectExt for JsObject {
  fn define_method<R, F>(&mut self, env: &Env, method_name: &str, method: F) -> napi::Result<()>
  where
    F: 'static + Fn(CallContext<'_>) -> napi::Result<R>,
    R: ToNapiValue,
  {
    let method_js = env.create_function_from_closure(method_name, method)?;
    self.set_named_property(method_name, method_js)?;

    Ok(())
  }
}
