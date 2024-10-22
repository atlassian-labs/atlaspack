use std::path::PathBuf;
use std::rc::Rc;

use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_napi_helpers::anyhow_to_napi;
use atlaspack_napi_helpers::JsObjectExt;
use napi::CallContext;
use napi::Env;
use napi::JsObject;
use napi::JsString;

pub struct NapiConfigLoader {
  config_loader: ConfigLoaderRef,
}

impl NapiConfigLoader {
  pub fn new(env: &Env, config_loader: ConfigLoaderRef) -> napi::Result<JsObject> {
    let this = Rc::new(Self { config_loader });
    let mut obj = env.create_object()?;

    obj.define_method(env, "getJsonConfigFrom", {
      let this = this.clone();
      move |ctx| this.get_json_config_from(ctx)
    })?;

    obj.define_method(env, "getJsonConfig", {
      let this = this.clone();
      move |ctx| this.get_json_config(ctx)
    })?;

    obj.define_method(env, "getPackageJson", {
      let this = this.clone();
      move |ctx| this.get_package_json(ctx)
    })?;

    Ok(obj)
  }

  fn get_json_config_from(&self, ctx: CallContext<'_>) -> napi::Result<Option<JsObject>> {
    let search_path_js = ctx.get::<JsString>(0)?;
    let search_path = search_path_js.into_utf8()?.into_owned()?;

    let file_names_js = ctx.get::<JsObject>(1)?;
    let mut file_names = vec![];

    for i in 0..file_names_js.get_array_length()? {
      let file_name_js = file_names_js.get_element::<JsString>(i)?;
      let file_name = file_name_js.into_utf8()?.into_owned()?;
      file_names.push(file_name);
    }

    let result = self
      .config_loader
      .get_json_config_from(&PathBuf::from(search_path), &file_names)
      .map_err(anyhow_to_napi)?;

    let Some(result) = result else {
      return Ok(None);
    };

    let mut result_js = ctx.env.create_object()?;

    result_js.set_named_property(
      "path",
      ctx.env.create_string(result.path.to_str().unwrap())?,
    )?;

    result_js.set_named_property(
      "contentRaw",
      ctx.env.to_js_value(&result.contents.to_string())?,
    )?;

    Ok(Some(result_js))
  }

  fn get_json_config(&self, _ctx: CallContext<'_>) -> napi::Result<()> {
    Ok(())
  }

  fn get_package_json(&self, _ctx: CallContext<'_>) -> napi::Result<()> {
    Ok(())
  }
}
