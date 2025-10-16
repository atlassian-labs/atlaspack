use atlassian_swc_compiled_css::apply_compiled_atomic_with_config;
use napi::Error as NapiError;

use napi::bindgen_prelude::Buffer;
use napi::{Env, JsObject};
use napi_derive::napi;
use std::path::Path;
use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::input::StringInput;
use swc_core::common::sync::Lrc;
use swc_core::common::{FileName, SourceMap};
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::codegen::{Config as CodegenConfig, Emitter};
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::{Parser, Syntax, TsSyntax};

// NAPI-compatible config struct, we duplicate this so the type can generate
#[napi(object)]
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

#[napi]
pub fn apply_compiled_css_in_js_plugin(
  env: Env,
  raw_code: Buffer,
  project_root: String,
  filename: String,
  _is_source: bool,
  config: CompiledCssInJsTransformConfig,
) -> napi::Result<JsObject> {
  // Convert Buffer to bytes properly
  let code_bytes = raw_code.as_ref();
  let code = std::str::from_utf8(code_bytes)
    .map_err(|e| NapiError::from_reason(format!("Input code is not valid UTF-8: {}", e)))?;

  let (deferred, promise) = env.create_deferred()?;
  let code_string = code.to_string();
  let filename_string = filename.clone();
  let project_root_string = project_root.clone();

  rayon::spawn(move || {
    let result = (|| -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
      // Create source map
      let source_map = Lrc::new(SourceMap::default());

      // Make filename relative to project root
      let relative_filename = Path::new(&filename_string)
        .strip_prefix(&project_root_string)
        .unwrap_or(Path::new(&filename_string));

      let source_file = source_map.new_source_file(
        Lrc::new(FileName::Real(relative_filename.to_path_buf())),
        code_string,
      );

      // Parse the code
      let comments = SingleThreadedComments::default();
      let syntax = Syntax::Typescript(TsSyntax {
        tsx: true,
        decorators: false,
        ..Default::default()
      });

      let lexer = Lexer::new(
        syntax,
        Default::default(),
        StringInput::from(&*source_file),
        Some(&comments),
      );

      let mut parser = Parser::new_from(lexer);
      let mut program = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {:?}", e))?;

      // Apply the transformation
      let internal_config: atlassian_swc_compiled_css::config::CompiledCssInJsTransformConfig =
        config.into();
      let _result = apply_compiled_atomic_with_config(&mut program, internal_config);

      // Emit the code
      let mut output_buffer = vec![];
      let writer = JsWriter::new(source_map.clone(), "\n", &mut output_buffer, None);

      let mut emitter = Emitter {
        cfg: CodegenConfig::default(),
        cm: source_map.clone(),
        comments: Some(&comments),
        wr: writer,
      };

      emitter
        .emit_program(&program)
        .map_err(|e| format!("Emit error: {:?}", e))?;

      Ok(output_buffer)
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
          "Failed to process Compiled CSS in JS: {}",
          e
        )));
      }
    }
  });

  Ok(promise)
}
