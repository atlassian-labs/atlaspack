use anyhow::{Context, Result, anyhow};
use atlaspack_js_swc_core::{
  Config, SourceType, emit, parse,
  utils::{ErrorBuffer, error_buffer_to_diagnostics},
};
use napi::{Env, Error as NapiError, JsObject, bindgen_prelude::Buffer};
use napi_derive::napi;
use swc_core::{
  common::{
    FileName, GLOBALS, SourceMap,
    errors::{self, Handler},
    source_map::SourceMapGenConfig,
    sync::Lrc,
  },
  ecma::ast::{Module, ModuleItem, Program},
};

// NAPI-compatible partial config struct for use from TypeScript
// All fields are optional and will be filled with defaults
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CompiledCssInJsConfig {
  pub import_react: Option<bool>,
  pub nonce: Option<String>,
  pub import_sources: Option<Vec<String>>,
  pub optimize_css: Option<bool>,
  pub extensions: Option<Vec<String>>,
  pub add_component_name: Option<bool>,
  pub process_xcss: Option<bool>,
  pub increase_specificity: Option<bool>,
  pub sort_at_rules: Option<bool>,
  pub sort_shorthand: Option<bool>,
  pub class_hash_prefix: Option<String>,
  pub flatten_multiple_selectors: Option<bool>,
  pub extract: Option<bool>,
  pub ssr: Option<bool>,
}

impl From<CompiledCssInJsConfig> for atlassian_swc_compiled_css::CompiledCssInJsTransformConfig {
  fn from(config: CompiledCssInJsConfig) -> Self {
    // Convert to the library's partial config type first
    let partial = atlassian_swc_compiled_css::CompiledCssInJsConfig {
      import_react: config.import_react,
      nonce: config.nonce,
      import_sources: config.import_sources,
      optimize_css: config.optimize_css,
      extensions: config.extensions,
      add_component_name: config.add_component_name,
      process_xcss: config.process_xcss,
      increase_specificity: config.increase_specificity,
      sort_at_rules: config.sort_at_rules,
      sort_shorthand: config.sort_shorthand,
      class_hash_prefix: config.class_hash_prefix,
      flatten_multiple_selectors: config.flatten_multiple_selectors,
      extract: config.extract,
      ssr: config.ssr,
    };
    // Then convert to full config with defaults
    partial.into()
  }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CompiledCssInJsPluginInput {
  pub filename: String,
  pub project_root: String,
  pub is_source: bool,
  pub source_maps: bool,
  pub config: CompiledCssInJsConfig,
}

#[napi(object)]
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledCssInJsPluginResult {
  pub code: String,
  pub map: Option<String>,
  pub style_rules: Vec<String>,
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
  // Check for empty code
  if code.trim().is_empty() {
    return Err(anyhow!("Empty code input"));
  }

  let swc_config = Config {
    is_type_script: true,
    is_jsx: true,
    decorators: false,
    source_type: SourceType::Module,
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

    // Convert the program to a module if it's a script
    let module = match module {
      Program::Module(module) => Program::Module(module),
      Program::Script(script) => Program::Module(Module {
        span: script.span,
        shebang: None,
        body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
      }),
    };

    // Convert config to the library's config type
    let lib_config: atlassian_swc_compiled_css::CompiledCssInJsTransformConfig =
      input.config.clone().into();

    // Apply the transformation using transform_program_with_config
    // This needs to be wrapped in GLOBALS context
    let (transformed_program, artifacts) = GLOBALS.set(&Default::default(), || {
      let transformed_program = atlassian_swc_compiled_css::transform_program_with_config(
        module,
        input.filename.clone(),
        lib_config,
      );

      // Get the collected style rules
      let artifacts = atlassian_swc_compiled_css::take_latest_artifacts();
      (transformed_program, artifacts)
    });

    let module_result = transformed_program
      .module()
      .ok_or_else(|| anyhow!("Failed to get transformed module"))?;
    let (code_bytes, line_pos_buffer) = emit(
      source_map.clone(),
      comments,
      &module_result,
      input.source_maps,
    )
    .with_context(|| "Failed to emit transformed code")?;

    let mut code =
      String::from_utf8(code_bytes).with_context(|| "Failed to convert emitted code to UTF-8")?;

    // Add the comment marker to indicate transformation occurred
    code = format!("/* COMPILED_TRANSFORMED_ASSET */\n{}", code);

    let map_json = if input.source_maps && !line_pos_buffer.is_empty() {
      let mut output_map_buffer = vec![];
      if source_map
        .build_source_map_with_config(&line_pos_buffer, None, SourceMapConfig)
        .to_writer(&mut output_map_buffer)
        .is_ok()
      {
        let map_string = String::from_utf8(output_map_buffer).unwrap_or_default();
        // Adjust the sourcemap to account for the added comment line
        // Parse the sourcemap, offset all generated line numbers by 1, and serialize it back
        if let Ok(mut map_value) = serde_json::from_str::<serde_json::Value>(&map_string) {
          if let Some(mappings) = map_value.get_mut("mappings") {
            if let Some(mappings_str) = mappings.as_str() {
              // VLQ sourcemap format: prepend a semicolon to shift all mappings down by 1 line
              *mappings = serde_json::Value::String(format!(";{}", mappings_str));
            }
          }
          Some(serde_json::to_string(&map_value).unwrap_or(map_string))
        } else {
          Some(map_string)
        }
      } else {
        None
      }
    } else {
      None
    };

    Ok(CompiledCssInJsPluginResult {
      code,
      map: map_json,
      style_rules: artifacts.style_rules,
    })
  })
}

#[napi]
pub fn apply_compiled_css_in_js_plugin(
  env: Env,
  raw_code: Buffer,
  input: CompiledCssInJsPluginInput,
) -> napi::Result<JsObject> {
  // Convert bytes to string and take ownership
  let code = std::str::from_utf8(raw_code.as_ref())
    .with_context(|| "Input code is not valid UTF-8")
    .map_err(|e| NapiError::from_reason(e.to_string()))?
    .to_string();

  // Return early for empty code
  if code.trim().is_empty() {
    return Err(NapiError::from_reason("Empty code input".to_string()));
  }

  // Create deferred promise
  let (deferred, promise) = env.create_deferred()?;

  // Spawn the work on a Rayon thread
  rayon::spawn(move || {
    let result = process_compiled_css_in_js(&code, &input);
    match result {
      Ok(plugin_result) => {
        deferred.resolve(move |env| env.to_js_value(&plugin_result));
      }
      Err(e) => {
        deferred.reject(NapiError::from_reason(e.to_string()));
      }
    }
  });

  Ok(promise)
}

#[cfg(test)]
mod tests {
  use super::*;
  use indoc::indoc;

  // Helper function to create test config
  fn create_test_config(source_maps: bool, extract: bool) -> CompiledCssInJsPluginInput {
    CompiledCssInJsPluginInput {
      filename: "test.tsx".to_string(),
      project_root: "/project".to_string(),
      is_source: false,
      source_maps,
      config: CompiledCssInJsConfig {
        import_react: Some(true),
        nonce: None,
        import_sources: Some(vec!["@compiled/react".into()]),
        optimize_css: Some(true),
        extensions: None,
        add_component_name: Some(false),
        process_xcss: Some(true),
        increase_specificity: Some(true),
        sort_at_rules: Some(true),
        class_hash_prefix: None,
        flatten_multiple_selectors: Some(true),
        extract: Some(extract),
        ssr: Some(true),
        sort_shorthand: Some(true),
      },
    }
  }

  #[test]
  fn test_successful_transformation() {
    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformed code should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.code.contains("@compiled/react/runtime"),
      "Transformed code should contain @compiled/react/runtime"
    );
  }

  #[test]
  fn test_comment_marker() {
    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformed code should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed
        .code
        .contains("/* COMPILED_TRANSFORMED_ASSET */"),
      "Transformed code should contain COMPILED_TRANSFORMED_ASSET comment marker"
    );
  }

  #[test]
  fn test_successful_extract_transformation() {
    let config = create_test_config(true, true);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformed code should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.code.contains("@compiled/react/runtime"),
      "Transformed code should contain @compiled/react/runtime"
    );
  }

  #[test]
  fn test_invalid_javascript_syntax_error() {
    let config = create_test_config(true, false);

    let invalid_code = indoc! {r#"
      import { css  from '@compiled/react'; // Missing closing brace
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(invalid_code, &config);
    assert!(result.is_err(), "Invalid syntax should result in an error");
    let error = result.unwrap_err();
    let error_string = error.to_string();
    // Just verify an error occurred - the exact error message format can vary
    assert!(
      error_string.contains("Parse error") || !error_string.is_empty(),
      "Expected some error message, got: {}",
      error_string
    );
  }

  #[test]
  fn test_typescript_syntax_support() {
    let config = create_test_config(true, false);

    let ts_code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      interface MyInterface {
        color: string;
      }
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(ts_code, &config);
    assert!(result.is_ok(), "TypeScript syntax should be supported");
  }

  #[test]
  fn test_empty_code_input() {
    let config = create_test_config(true, false);

    let empty_code = "";
    let result = process_compiled_css_in_js(empty_code, &config);

    // Empty code should result in an error since we now check for it
    assert!(result.is_err(), "Empty code should result in an error");
  }

  #[test]
  fn test_code_without_compiled() {
    let config = create_test_config(true, false);

    let code_without_compiled = indoc! {r#"
      const greeting = "Hello, world!";
      console.log(greeting);
    "#};

    let result = process_compiled_css_in_js(code_without_compiled, &config);

    // Code without compiled should still be processed successfully
    // Even if there are no compiled components to transform, the code should parse and emit correctly
    match result {
      Ok(transformed) => {
        assert!(
          transformed.code.contains("Hello, world!"),
          "Original code should be preserved"
        );
      }
      Err(e) => {
        panic!("Should not get an error; got error for code: {}", e);
      }
    }
  }

  #[test]
  fn test_sourcemap_generation() {
    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformed code should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.code.contains("@compiled/react/runtime"),
      "Transformed code should contain @compiled/react/runtime"
    );

    // Check that sourcemap is generated
    assert!(transformed.map.is_some(), "Sourcemap should be generated");

    let map = transformed.map.unwrap();
    assert!(!map.is_empty(), "Sourcemap should not be empty");

    // Verify that the sourcemap is valid JSON
    let parsed_sourcemap: Result<serde_json::Value, _> = serde_json::from_str(&map);
    assert!(parsed_sourcemap.is_ok(), "Sourcemap should be valid JSON");
  }

  #[test]
  fn test_no_sourcemap_when_disabled() {
    let config = create_test_config(false, false);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Token transformation should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.code.contains("token"),
      "Transformed code should contain token reference"
    );

    // Check that sourcemap is NOT generated when disabled
    assert!(
      transformed.map.is_none(),
      "Sourcemap should not be generated when source_maps is false"
    );
  }
}
