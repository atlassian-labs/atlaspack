use atlaspack_js_swc_core::{Config, emit, parse};
use napi::bindgen_prelude::Buffer;
use napi::{Env, Error as NapiError, JsObject};
use napi_derive::napi;
use swc_atlaskit_tokens::design_system_tokens_visitor;
use swc_atlaskit_tokens::token_map::get_or_load_token_map_from_json;
use swc_core::common::SourceMap;
use swc_core::common::sync::Lrc;

#[napi]
pub fn apply_tokens_plugin(
  raw_code: Buffer,
  project_root: String,
  filename: String,
  is_source: bool,
  tokens_path: String,
  env: Env,
) -> napi::Result<JsObject> {
  let config = Config {
    is_type_script: true,
    is_jsx: true,
    decorators: false,
    ..Default::default()
  };

  // Convert Buffer to bytes properly
  let code_bytes = raw_code.as_ref();
  let code = std::str::from_utf8(code_bytes)
    .map_err(|e| NapiError::from_reason(format!("Input code is not valid UTF-8: {}", e)))?;

  let (deferred, promise) = env.create_deferred()?;
  let code_string = code.to_string();

  rayon::spawn(move || {
    let result = (|| -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
      let source_map = Lrc::new(SourceMap::default());
      let (module, comments) = parse(&code_string, &project_root, &filename, &source_map, &config)
        .map_err(|e| format!("Parse error: {:?}", e))?;

      let token_map = get_or_load_token_map_from_json(Some(&tokens_path))?;

      // FIXME load the config from config
      let mut passes = design_system_tokens_visitor(
        comments.clone(),
        true,
        false,
        vec![],
        "light".to_string(),
        !is_source,
        token_map.as_ref().map(|t| t.as_ref()),
      );
      let module = module.apply(&mut passes);

      let module_result = module.module().ok_or("Failed to get module")?;
      let (code, _) = emit(source_map, comments, &module_result, false)?;
      Ok(code)
    })();

    match result {
      Ok(code_bytes) => {
        deferred.resolve(move |env| {
          env
            .create_buffer_with_data(code_bytes)
            .map(|buf| buf.into_raw())
        });
      }
      Err(e) => {
        deferred.reject(NapiError::from_reason(format!(
          "Failed to process tokens: {}",
          e
        )));
      }
    }
  });

  Ok(promise)
}
