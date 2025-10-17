use anyhow::{Context, Result, anyhow};
use atlaspack_js_swc_core::{
  Config, emit, parse, utils::ErrorBuffer, utils::error_buffer_to_diagnostics,
};
use atlassian_swc_compiled_css::compiled_css_in_js_visitor;
use napi::{Env, Error as NapiError, JsObject, bindgen_prelude::Buffer};
use napi_derive::napi;
use swc_core::common::{
  FileName, SourceMap, errors, errors::Handler, source_map::SourceMapGenConfig, sync::Lrc,
};

// NAPI-compatible config struct, we duplicate this so the type can generate
#[napi(object)]
#[derive(Clone, Debug)]
pub struct CompiledCssInJsTransformConfig {
  pub import_react: Option<bool>,
  pub nonce: Option<String>,
  pub import_sources: Option<Vec<String>>,
  pub optimize_css: Option<bool>,
  pub extensions: Option<Vec<String>>,
  pub add_component_name: Option<bool>,
  pub process_xcss: Option<bool>,
  pub increase_specificity: Option<bool>,
  pub sort_at_rules: Option<bool>,
  pub class_hash_prefix: Option<String>,
  pub flatten_multiple_selectors: Option<bool>,
  pub extract: Option<bool>,
  pub ssr: Option<bool>,
}

impl From<CompiledCssInJsTransformConfig>
  for atlassian_swc_compiled_css::config::CompiledCssInJsTransformConfig
{
  fn from(config: CompiledCssInJsTransformConfig) -> Self {
    Self {
      import_react: config.import_react,
      nonce: config.nonce,
      import_sources: config.import_sources,
      optimize_css: config.optimize_css,
      extensions: config.extensions,
      add_component_name: config.add_component_name,
      process_xcss: config.process_xcss,
      increase_specificity: config.increase_specificity,
      sort_at_rules: config.sort_at_rules,
      class_hash_prefix: config.class_hash_prefix,
      flatten_multiple_selectors: config.flatten_multiple_selectors,
      extract: config.extract,
      ssr: config.ssr,
    }
  }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CompiledCssInJsPluginInput {
  pub filename: String,
  pub project_root: String,
  pub is_source: bool,
  pub source_maps: bool,
  pub config: CompiledCssInJsTransformConfig,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CompiledCssInJsPluginResult {
  pub code: String,
  pub map: Option<String>,
}

// Exclude macro expansions from source maps.
struct SourceMapConfig;
impl SourceMapGenConfig for SourceMapConfig {
  fn file_name_to_source(&self, f: &FileName) -> String {
    f.to_string()
  }

  fn skip(&self, f: &FileName) -> bool {
    matches!(f, FileName::MacroExpansion | FileName::Internal(..))
  }
}

fn process_compiled_css_in_js(
  code: &str,
  input: &CompiledCssInJsPluginInput,
) -> Result<CompiledCssInJsPluginResult> {
  let swc_config = Config {
    is_type_script: true,
    is_jsx: true,
    decorators: false,
    ..Default::default()
  };

  let error_buffer = ErrorBuffer::default();
  let handler = Handler::with_emitter(true, false, Box::new(error_buffer.clone()));
  errors::HANDLER.set(&handler, || {
    let source_map = Lrc::new(SourceMap::default());

    // Parse and handle parsing errors
    let (module, comments) = match parse(
      code,
      &input.project_root,
      &input.filename,
      &source_map,
      &swc_config,
    ) {
      Ok(result) => result,
      Err(_parsing_errors) => {
        let diagnostics = error_buffer_to_diagnostics(&error_buffer, &source_map);
        let error_msg = diagnostics
          .iter()
          .map(|d| &d.message)
          .cloned()
          .collect::<Vec<_>>()
          .join("\n");
        return Err(anyhow!("Parse error: {}", error_msg));
      }
    };

    let config: atlassian_swc_compiled_css::CompiledCssInJsTransformConfig =
      <atlassian_swc_compiled_css::CompiledCssInJsTransformConfig>::from(input.config.clone());
    let mut passes = compiled_css_in_js_visitor(&config);
    let module = module.apply(&mut passes);

    let module_result = module
      .module()
      .ok_or_else(|| anyhow!("Failed to get transformed module"))?;
    let (code_bytes, line_pos_buffer) = emit(
      source_map.clone(),
      comments,
      &module_result,
      input.source_maps,
    )
    .with_context(|| "Failed to emit transformed code")?;

    let code =
      String::from_utf8(code_bytes).with_context(|| "Failed to convert emitted code to UTF-8")?;
    let map_json = if input.source_maps && !line_pos_buffer.is_empty() {
      let mut output_map_buffer = vec![];
      if source_map
        .build_source_map_with_config(&line_pos_buffer, None, SourceMapConfig)
        .to_writer(&mut output_map_buffer)
        .is_ok()
      {
        Some(String::from_utf8(output_map_buffer).unwrap_or_default())
      } else {
        None
      }
    } else {
      None
    };

    Ok(CompiledCssInJsPluginResult {
      code,
      map: map_json,
    })
  })
}

#[napi]
pub fn apply_compiled_css_in_js_plugin(
  env: Env,
  raw_code: Buffer,
  input: CompiledCssInJsPluginInput,
) -> napi::Result<JsObject> {
  let code_bytes = raw_code.as_ref();

  // Convert bytes to string and create owned copy for moving into closure
  let code = std::str::from_utf8(code_bytes)
    .with_context(|| "Input code is not valid UTF-8")
    .map_err(|e| NapiError::from_reason(e.to_string()))?
    .to_string();

  // Return early for empty code
  if code.trim().is_empty() {
    return Err(NapiError::from_reason("Empty code input".to_string()));
  }

  let (deferred, promise) = env.create_deferred()?;
  rayon::spawn(move || {
    let result = process_compiled_css_in_js(&code, &input);

    match result {
      Ok(result) => {
        deferred.resolve(move |_env| Ok(result));
      }
      Err(e) => {
        deferred.reject(NapiError::from_reason(format!(
          "Failed to process Compiled CSS in JS: {}",
          e
        )));
      }
    }
  });

  Ok(promise)
}
