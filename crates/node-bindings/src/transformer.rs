use napi::Env;
use napi::JsObject;
use napi::JsUnknown;
use napi_derive::napi;
use std::path::Path;

#[napi]
pub fn transform(opts: JsObject, env: Env) -> napi::Result<JsUnknown> {
  let config: atlaspack_js_swc_core::Config = env.from_js_value(opts)?;

  let result = atlaspack_js_swc_core::transform(&config, None)?;
  env.to_js_value(&result)
}

#[napi]
pub fn determine_jsx_configuration(
  file_path: String,
  is_source: bool,
  config: JsObject,
  project_root: String,
  env: Env,
) -> napi::Result<JsUnknown> {
  let config: Option<atlaspack_js_swc_core::JsxOptions> = env.from_js_value(config)?;
  let file_path = Path::new(&file_path);
  let project_root = Path::new(&project_root);
  let file_type = &file_path
    .extension()
    .map_or("", |ext| ext.to_str().unwrap_or(""));

  let result = atlaspack_js_swc_core::determine_jsx_configuration(
    file_path,
    file_type,
    is_source,
    &config,
    project_root,
  );

  env.to_js_value(&result)
}

#[cfg(not(target_arch = "wasm32"))]
mod native_only {
  use atlaspack_macros::napi::create_macro_callback;

  use super::*;

  #[napi]
  pub fn transform_async(opts: JsObject, env: Env) -> napi::Result<JsObject> {
    let call_macro = if opts.has_named_property("callMacro")? {
      let func = opts.get_named_property::<JsUnknown>("callMacro")?;
      if let Ok(func) = func.try_into() {
        Some(create_macro_callback(func, env)?)
      } else {
        None
      }
    } else {
      None
    };

    let config: atlaspack_js_swc_core::Config = env.from_js_value(opts)?;
    let (deferred, promise) = env.create_deferred()?;

    rayon::spawn(move || {
      let res = atlaspack_js_swc_core::transform(&config, call_macro);
      match res {
        Ok(result) => deferred.resolve(move |env| env.to_js_value(&result)),
        Err(err) => deferred.reject(err.into()),
      }
    });

    Ok(promise)
  }
}
