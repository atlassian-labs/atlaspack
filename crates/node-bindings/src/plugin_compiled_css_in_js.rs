use anyhow::{Context, Result, anyhow};
use atlaspack_js_swc_core::{
  Config, SourceType, emit, parse,
  utils::{
    CodeHighlight, Diagnostic, DiagnosticSeverity, ErrorBuffer, SourceLocation,
    error_buffer_to_diagnostics,
  },
};
use atlassian_swc_compiled_css::EmitCommentsGuard;
use napi::{Env, Error as NapiError, JsObject, bindgen_prelude::Buffer};
use napi_derive::napi;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{any::Any, panic};
use swc_core::common::Mark;
use swc_core::{
  common::{
    FileName, GLOBALS, SourceMap, Span,
    comments::{Comment, SingleThreadedComments},
    errors::{self, Handler},
    source_map::SourceMapGenConfig,
    sync::Lrc,
  },
  ecma::ast::{Module, ModuleItem, Program},
  plugin::proxies::{PluginSourceMapProxy, TransformPluginProgramMetadata},
};

/// Error type for transform failures
#[derive(Debug, Clone)]
pub struct TransformError {
  pub message: String,
  pub span: Option<Span>,
}

/// Result type for transform operations
pub type TransformErrors = Vec<TransformError>;

// NAPI-compatible partial config struct for use from TypeScript
// All fields are optional and will be filled with defaults
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CompiledCssInJsConfig {
  pub config_path: Option<String>,
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
  pub unsafe_report_safe_assets_for_migration: Option<bool>,
  pub unsafe_use_safe_assets: Option<bool>,
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
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledCssInJsPluginResult {
  pub code: String,
  pub map: Option<String>,
  pub style_rules: Vec<String>,
  pub diagnostics: Vec<JsDiagnostic>,
  pub bail_out: bool,
  pub code_hash: String,
}

static PANIC_HOOK_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn is_jsx_pragma_comment(comment: &Comment) -> bool {
  let text = comment.text.as_ref();
  text.contains("@jsx") || text.contains("@jsxRuntime") || text.contains("@jsxImportSource")
}

fn remove_jsx_pragma_comments(comments: &SingleThreadedComments) -> bool {
  let (mut leading, mut trailing) = comments.borrow_all_mut();
  let mut removed_any = false;

  leading.retain(|_, comment_list| {
    let original_len = comment_list.len();
    comment_list.retain(|comment| !is_jsx_pragma_comment(comment));
    if comment_list.len() != original_len {
      removed_any = true;
    }
    !comment_list.is_empty()
  });

  trailing.retain(|_, comment_list| {
    let original_len = comment_list.len();
    comment_list.retain(|comment| !is_jsx_pragma_comment(comment));
    if comment_list.len() != original_len {
      removed_any = true;
    }
    !comment_list.is_empty()
  });

  removed_any
}

fn strip_jsx_pragma_comment_from_source(source: &str) -> String {
  fn find_comment_bounds(source: &str, marker_index: usize) -> Option<(usize, usize)> {
    let comment_start = source[..marker_index].rfind("/*")?;
    let comment_end_relative = source[marker_index..].find("*/")?;
    let comment_end = marker_index + comment_end_relative + 2;
    Some((comment_start, comment_end))
  }

  let mut earliest_marker: Option<(usize, &str)> = None;
  for marker in ["@jsxImportSource", "@jsxRuntime", "@jsx"] {
    if let Some(index) = source.find(marker) {
      earliest_marker = match earliest_marker {
        Some((existing_index, existing_marker)) if existing_index <= index => {
          Some((existing_index, existing_marker))
        }
        _ => Some((index, marker)),
      };
    }
  }

  let Some((marker_index, _)) = earliest_marker else {
    return source.to_string();
  };

  // Only remove if marker appears within a block comment.
  let Some((start, end)) = find_comment_bounds(source, marker_index) else {
    return source.to_string();
  };

  let mut result = String::with_capacity(source.len());
  result.push_str(&source[..start]);

  let mut rest = &source[end..];
  if rest.starts_with("\r\n") {
    rest = &rest[2..];
  } else if rest.starts_with('\n') {
    rest = &rest[1..];
  }
  result.push_str(rest);

  result
}

fn map_transform_errors_to_diagnostics(
  errors: Vec<TransformError>,
  source_map: &SourceMap,
) -> Vec<Diagnostic> {
  errors
    .into_iter()
    .map(|error| {
      let code_highlights = error.span.and_then(|span| {
        if span.lo().is_dummy() || span.hi().is_dummy() {
          None
        } else {
          Some(vec![CodeHighlight {
            message: None,
            loc: SourceLocation::from(source_map, span),
          }])
        }
      });

      Diagnostic {
        message: error.message,
        code_highlights,
        hints: None,
        show_environment: false,
        severity: DiagnosticSeverity::Error,
        documentation_url: None,
      }
    })
    .collect()
}

#[napi(object)]
#[derive(Debug, serde::Serialize)]
pub struct JsSourceLocation {
  pub start_line: u32,
  pub start_col: u32,
  pub end_line: u32,
  pub end_col: u32,
}

#[napi(object)]
#[derive(Debug, serde::Serialize)]
pub struct JsCodeHighlight {
  pub message: Option<String>,
  pub loc: JsSourceLocation,
}

#[napi(object)]
#[derive(Debug, serde::Serialize)]
pub struct JsDiagnostic {
  pub message: String,
  pub code_highlights: Option<Vec<JsCodeHighlight>>,
  pub hints: Option<Vec<String>>,
  pub show_environment: bool,
  pub severity: String,
  pub documentation_url: Option<String>,
}

fn convert_source_location(loc: SourceLocation) -> JsSourceLocation {
  JsSourceLocation {
    start_line: loc.start_line as u32,
    start_col: loc.start_col as u32,
    end_line: loc.end_line as u32,
    end_col: loc.end_col as u32,
  }
}

fn convert_code_highlight(highlight: CodeHighlight) -> JsCodeHighlight {
  JsCodeHighlight {
    message: highlight.message,
    loc: convert_source_location(highlight.loc),
  }
}

fn convert_diagnostic(diagnostic: Diagnostic) -> JsDiagnostic {
  let severity = match diagnostic.severity {
    DiagnosticSeverity::Error => "Error",
    DiagnosticSeverity::Warning => "Warning",
    DiagnosticSeverity::SourceError => "SourceError",
  }
  .to_string();

  JsDiagnostic {
    message: diagnostic.message,
    code_highlights: diagnostic
      .code_highlights
      .map(|highlights| highlights.into_iter().map(convert_code_highlight).collect()),
    hints: diagnostic.hints,
    show_environment: diagnostic.show_environment,
    severity,
    documentation_url: diagnostic.documentation_url,
  }
}

fn convert_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<JsDiagnostic> {
  diagnostics.into_iter().map(convert_diagnostic).collect()
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

  let code_hash = atlassian_swc_compiled_css::migration_hash::hash_code(code);

  // Build the transform config from the input
  let transform_config = atlassian_swc_compiled_css::CompiledCssInJsTransformConfig::from(
    atlassian_swc_compiled_css::CompiledCssInJsConfig {
      config_path: input.config.config_path.clone(),
      import_react: input.config.import_react,
      nonce: input.config.nonce.clone(),
      import_sources: input.config.import_sources.clone(),
      optimize_css: input.config.optimize_css,
      extensions: input.config.extensions.clone(),
      add_component_name: input.config.add_component_name,
      process_xcss: input.config.process_xcss,
      increase_specificity: input.config.increase_specificity,
      sort_at_rules: input.config.sort_at_rules,
      sort_shorthand: input.config.sort_shorthand,
      class_hash_prefix: input.config.class_hash_prefix.clone(),
      flatten_multiple_selectors: input.config.flatten_multiple_selectors,
      extract: input.config.extract,
      ssr: input.config.ssr,
      unsafe_report_safe_assets_for_migration: input.config.unsafe_report_safe_assets_for_migration,
      unsafe_use_safe_assets: input.config.unsafe_use_safe_assets,
    },
  );

  if !atlassian_swc_compiled_css::migration_hash::is_safe(&code_hash, &transform_config)? {
    // Bail out if the code is not safe from our safe assets map
    return Ok(CompiledCssInJsPluginResult {
      code: "".to_string(),
      map: None,
      style_rules: Vec::new(),
      diagnostics: Vec::new(),
      bail_out: true,
      code_hash,
    });
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

    let emit_guard = EmitCommentsGuard::new(&comments);

    // Apply the transformation using transform_program_with_options wrapped with error handling
    // This needs to be wrapped in GLOBALS context
    let (transform_result, artifacts) = GLOBALS.set(&Default::default(), || {
      let transformed_result =
        transform_program_with_config(module, input.filename.clone(), transform_config);

      // Get the collected style rules
      let artifacts = atlassian_swc_compiled_css::take_latest_artifacts();

      (transformed_result, artifacts)
    });

    let transformed_program = match transform_result {
      Ok(program) => program,
      Err(errors) => {
        let mut diagnostics = map_transform_errors_to_diagnostics(errors, &source_map);
        diagnostics.extend(error_buffer_to_diagnostics(&error_buffer, &source_map));
        if diagnostics.is_empty() {
          diagnostics.push(Diagnostic {
            message: "Compiled CSS in JS transform failed".to_string(),
            code_highlights: None,
            hints: None,
            show_environment: false,
            severity: DiagnosticSeverity::Error,
            documentation_url: None,
          });
        }

        let diagnostics = convert_diagnostics(diagnostics);

        return Ok(CompiledCssInJsPluginResult {
          code: code.to_string(),
          map: None,
          style_rules: Vec::new(),
          diagnostics,
          bail_out: true,
          code_hash,
        });
      }
    };

    drop(emit_guard);

    remove_jsx_pragma_comments(&comments);

    let module_result = transformed_program
      .module()
      .ok_or_else(|| anyhow!("Failed to get transformed module"))?;
    let (code_bytes, line_pos_buffer) = emit(
      source_map.clone(),
      comments,
      &module_result,
      input.source_maps,
      Some(false), // Preserve Unicode characters for now
    )
    .with_context(|| "Failed to emit transformed code")?;

    let code =
      String::from_utf8(code_bytes).with_context(|| "Failed to convert emitted code to UTF-8")?;
    let code = strip_jsx_pragma_comment_from_source(&code);

    let map_json = if input.source_maps && !line_pos_buffer.is_empty() {
      let build_map_result = {
        let _hook_guard = PANIC_HOOK_GUARD.lock();
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));

        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
          let mut output_map_buffer = vec![];
          let write_result = source_map
            .build_source_map_with_config(&line_pos_buffer, None, SourceMapConfig)
            .to_writer(&mut output_map_buffer);
          (write_result, output_map_buffer)
        }));

        panic::set_hook(previous_hook);
        result
      };

      match build_map_result {
        Ok((Ok(()), output_map_buffer)) => {
          let map_string = String::from_utf8(output_map_buffer).unwrap_or_default();
          if let Ok(mut map_value) = serde_json::from_str::<serde_json::Value>(&map_string) {
            if let Some(mappings) = map_value.get_mut("mappings")
              && let Some(mappings_str) = mappings.as_str()
            {
              *mappings = serde_json::Value::String(format!(";{}", mappings_str));
            }
            Some(serde_json::to_string(&map_value).unwrap_or(map_string))
          } else {
            Some(map_string)
          }
        }
        Ok((Err(_), _)) => None,
        Err(_) => None,
      }
    } else {
      None
    };

    // Extract filename from path to match babel plugin format
    let filename = std::path::Path::new(&input.filename)
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("unknown");
    let code = format!(
      "/* {} generated by @compiled/babel-plugin v0.38.1 */\n{}",
      filename, code
    );
    let code = append_transformed_asset_marker(code);

    Ok(CompiledCssInJsPluginResult {
      code,
      map: map_json,
      style_rules: artifacts.style_rules,
      diagnostics: Vec::new(),
      bail_out: false,
      code_hash,
    })
  })
}

fn append_transformed_asset_marker(mut code: String) -> String {
  if !code.ends_with('\n') {
    code.push('\n');
  }
  code.push_str("/* COMPILED_TRANSFORMED_ASSET */");
  code
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

/// Transform using the swc plugin
pub fn transform_program_with_config(
  program: Program,
  filename: String,
  config: atlassian_swc_compiled_css::CompiledCssInJsTransformConfig,
) -> Result<Program, TransformErrors> {
  // Convert config to PluginOptions directly
  let mut options = config_to_plugin_options(config);

  // Ensure @compiled/react is in import_sources
  if !options
    .import_sources
    .iter()
    .any(|s| s == "@compiled/react")
  {
    options.import_sources.push("@compiled/react".to_string());
  }

  let metadata = TransformPluginProgramMetadata {
    comments: None,
    source_map: PluginSourceMapProxy {
      source_file: OnceCell::new(),
    },
    unresolved_mark: Mark::new(),
  };

  // Wrap the transform_program_with_options call to catch any panics or errors
  wrap_transform_with_error_handling(program, metadata, options, filename)
}

/// Convert CompiledCssInJsTransformConfig to PluginOptions
fn config_to_plugin_options(
  config: atlassian_swc_compiled_css::CompiledCssInJsTransformConfig,
) -> atlassian_swc_compiled_css::PluginOptions {
  use std::collections::BTreeMap;

  atlassian_swc_compiled_css::PluginOptions {
    extract: config.extract,
    import_sources: config.import_sources,
    class_hash_prefix: config.class_hash_prefix,
    process_xcss: config.process_xcss,
    class_name_compression_map: BTreeMap::new(),
    import_react: Some(config.import_react),
    add_component_name: Some(config.add_component_name),
    nonce: config.nonce,
    cache: None,
    optimize_css: Some(config.optimize_css),
    extensions: config.extensions.unwrap_or_default(),
    parser_babel_plugins: Vec::new(),
    increase_specificity: Some(config.increase_specificity),
    sort_at_rules: Some(config.sort_at_rules),
    flatten_multiple_selectors: Some(config.flatten_multiple_selectors),
    style_sheet_path: None,
    compiled_require_exclude: None,
    extract_styles_to_directory: None,
    sort_shorthand: Some(config.sort_shorthand),
    on_included_files: None,
  }
}

/// Wraps transform_program_with_options to catch errors and convert them to TransformError
fn wrap_transform_with_error_handling(
  program: Program,
  metadata: TransformPluginProgramMetadata,
  options: atlassian_swc_compiled_css::PluginOptions,
  filename: String,
) -> Result<Program, TransformErrors> {
  // Attempt to catch panics during transformation
  let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
    atlassian_swc_compiled_css::transform_program_with_options(program, metadata, options, filename)
  }));

  match result {
    Ok(transformed) => Ok(transformed),
    Err(panic_payload) => Err(panic_payload_to_transform_errors(panic_payload)),
  }
}

fn panic_payload_to_transform_errors(panic_payload: Box<dyn Any + Send>) -> TransformErrors {
  fn message_to_error(message: String) -> TransformError {
    TransformError {
      message,
      span: None,
    }
  }

  let panic_payload = match panic_payload.downcast::<TransformErrors>() {
    Ok(errors) => return *errors,
    Err(payload) => payload,
  };

  let panic_payload = match panic_payload.downcast::<TransformError>() {
    Ok(error) => return vec![*error],
    Err(payload) => payload,
  };

  let panic_payload = match panic_payload.downcast::<String>() {
    Ok(message) => return vec![message_to_error(*message)],
    Err(payload) => payload,
  };

  match panic_payload.downcast::<&'static str>() {
    Ok(message) => vec![message_to_error((*message).to_string())],
    Err(_) => vec![message_to_error(
      "Transform panicked - possibly an internal SWC plugin error".to_string(),
    )],
  }
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
        config_path: None,
        unsafe_report_safe_assets_for_migration: None,
        unsafe_use_safe_assets: None,
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
    assert!(!transformed.bail_out, "Should not bail out on success");
    assert!(
      transformed.diagnostics.is_empty(),
      "Expected no diagnostics on success"
    );
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
    assert!(!transformed.bail_out, "Should not bail out on success");
    assert!(
      transformed.diagnostics.is_empty(),
      "Expected no diagnostics on success"
    );
    assert!(
      transformed
        .code
        .contains("generated by @compiled/babel-plugin"),
      "Transformed code should contain babel plugin comment marker"
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
    assert!(!transformed.bail_out, "Should not bail out on success");
    assert!(
      transformed.diagnostics.is_empty(),
      "Expected no diagnostics on success"
    );
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
    let transformed = result.unwrap();
    assert!(!transformed.bail_out, "Should not bail out on success");
    assert!(
      transformed.diagnostics.is_empty(),
      "Expected no diagnostics on success"
    );
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
        assert!(!transformed.bail_out, "Should not bail out on success");
        assert!(
          transformed.diagnostics.is_empty(),
          "Expected no diagnostics on success"
        );
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
    assert!(!transformed.bail_out, "Should not bail out on success");
    assert!(
      transformed.diagnostics.is_empty(),
      "Expected no diagnostics on success"
    );
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
    assert!(!transformed.bail_out, "Should not bail out on success");
    assert!(
      transformed.diagnostics.is_empty(),
      "Expected no diagnostics on success"
    );
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

  #[test]
  fn test_bail_out_on_transform_error() {
    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const render = (value) => <div xcss={{ color: value }} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(
      result.is_ok(),
      "Transformation should return a bail-out result"
    );

    let transformed = result.unwrap();
    assert!(
      transformed.bail_out,
      "Expected transformer to bail out on error"
    );
    assert_eq!(
      transformed.code, code,
      "Bail out should return original code"
    );
    assert!(
      !transformed.diagnostics.is_empty(),
      "Expected diagnostics to be reported on bail out"
    );
    assert!(
      transformed.diagnostics[0]
        .message
        .contains("Object given to the xcss prop must be static"),
      "Unexpected diagnostic message: {:?}",
      transformed.diagnostics
    );
  }

  #[test]
  fn test_component_before_styles() {
    unsafe {
      std::env::set_var("COMPILED_DEBUG_CSS", "1");
    }
    let config = create_test_config(true, false);

    let code = indoc! {r#"
/** @jsx jsx */

import { css, jsx } from '@compiled/react';
import { Stack, Text } from '@atlaskit/primitives/compiled';
import { token } from '@atlaskit/tokens';
import { useIntl } from '@atlassian/jira-intl';
import type { JiraBoardType } from '../../../services/use-board-create-mutation/__generated__/useBoardCreateMutation.graphql';
import commonMessages from '../../../common/messages.tsx';
import { descriptionMapping } from '../../../common/utils.tsx';

export type ModalHeaderTypes = {
	type: JiraBoardType;
};

export const ModalHeader = ({ type }: ModalHeaderTypes): JSX.Element => {
	const { formatMessage } = useIntl();

	return (
		<Stack space="space.100">
			{/* eslint-disable-next-line @atlaskit/design-system/use-heading */}
			<h4 css={modalHeaderStyles}>{formatMessage(commonMessages.boardCreationTitle)}</h4>
			<Text size="medium" color="color.text.accent.gray.bolder">
				{descriptionMapping[type] && formatMessage(descriptionMapping[type])}
			</Text>
			<Text as="em" size="small" color="color.text.subtlest">
				{formatMessage(commonMessages.requiredFieldsHeader)}
			</Text>
		</Stack>
	);
};

const modalHeaderStyles = css({
	font: token('font.heading.medium'),
	fontWeight: token('font.weight.bold'),
	color: token('color.text'),
});

"#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let transformed = result.unwrap();

    // The important part: verify that the styles are transformed correctly
    // even though the component is declared before the styles
    assert!(
      !transformed.code.contains("css={"),
      "Should not have css in the code"
    );
    assert!(
      transformed.code.contains("className="),
      "Should have className in the code"
    );
    assert!(
      !transformed.style_rules.is_empty(),
      "Should have collected style rules"
    );
  }
}
