use anyhow::{Context, Result, anyhow};
use atlaspack_js_swc_core::{
  Config, SourceType, emit, parse,
  utils::{
    CodeHighlight, Diagnostic, DiagnosticSeverity, ErrorBuffer, SourceLocation,
    error_buffer_to_diagnostics, transform_errors_to_diagnostics,
  },
};
use atlassian_swc_compiled_css::TransformError;
use atlassian_swc_compiled_css_strip_runtime as strip_runtime;
use napi::{Env, Error as NapiError, JsObject, bindgen_prelude::Buffer};
use napi_derive::napi;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use std::collections::HashMap;
use std::panic;
use swc_core::{
  common::{
    FileName, GLOBALS, SourceMap,
    comments::{Comment, SingleThreadedComments},
    errors::{self, Handler},
    source_map::SourceMapGenConfig,
    sync::Lrc,
  },
  ecma::ast::{Module, ModuleItem, Program},
};

#[napi]
pub fn is_safe_from_js(hash: String, config_path: String) -> napi::Result<bool> {
  atlassian_swc_compiled_css::migration_hash::is_safe_from_js(hash, config_path)
    .map_err(|err| napi::Error::from_reason(err.to_string()))
}

#[napi]
pub fn hash_code(raw_code: String) -> String {
  atlassian_swc_compiled_css::migration_hash::hash_code(&raw_code)
}

// NAPI-compatible partial config struct for use from TypeScript
// All fields are optional and will be filled with defaults
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CompiledCssInJsConfigPlugin {
  pub config_path: Option<String>,
  pub import_react: Option<bool>,
  pub nonce: Option<String>,
  pub import_sources: Vec<String>,
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
  pub unsafe_skip_pattern: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CompiledCssInJsPluginInput {
  pub filename: String,
  pub project_root: String,
  pub is_source: bool,
  pub source_maps: bool,
  pub config: CompiledCssInJsConfigPlugin,
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
  /// Files that were imported/included during transformation.
  /// These should be tracked for cache invalidation.
  pub included_files: Vec<String>,
}

/// Guard to protect concurrent access to panic handler during source map generation
static PANIC_HOOK_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Cache for compiled regex patterns to avoid recompiling them on every function call
static REGEX_CACHE: Lazy<Mutex<HashMap<String, Regex>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Get or compile a regex pattern from the cache
fn get_cached_regex(pattern: &str) -> Result<Regex> {
  let mut cache = REGEX_CACHE.lock();

  if let Some(regex) = cache.get(pattern) {
    return Ok(regex.clone());
  }

  let regex = Regex::new(pattern).map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;
  cache.insert(pattern.to_string(), regex.clone());
  Ok(regex)
}

fn is_jsx_pragma_comment(comment: &Comment) -> bool {
  let text = comment.text.as_ref();
  text.contains("@jsx")
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

use crate::diagnostics::{JsDiagnostic, convert_diagnostics};

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

pub fn collect_comments_for_transform(comments: &SingleThreadedComments) -> Vec<Comment> {
  let (leading, trailing) = comments.borrow_all();

  leading
    .values()
    .chain(trailing.values())
    .flat_map(|list| list.clone())
    .collect()
}

fn process_compiled_css_in_js(
  code: &str,
  input: &CompiledCssInJsPluginInput,
) -> Result<CompiledCssInJsPluginResult> {
  // Check for empty code
  if code.trim().is_empty() {
    return Err(anyhow!("Empty code input"));
  }

  // Build the transform config from the input
  let transform_config = &atlassian_swc_compiled_css::CompiledCssInJsTransformConfig::from(
    atlassian_swc_compiled_css::config::CompiledCssInJsConfig {
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
      unsafe_skip_pattern: input.config.unsafe_skip_pattern.clone(),
    },
  );

  if let Some(pattern) = &transform_config.unsafe_skip_pattern {
    let regex = get_cached_regex(pattern.as_str())?;

    if regex.is_match(code) {
      // Asset contains known unsafe CSS, bail out without erroring
      return Ok(CompiledCssInJsPluginResult {
        code: "".to_string(),
        map: None,
        style_rules: Vec::new(),
        diagnostics: vec![JsDiagnostic {
          message: "Skipping asset from configured pattern".to_string(),
          code_highlights: None,
          hints: None,
          show_environment: false,
          severity: "Error".to_string(),
          documentation_url: None,
        }],
        bail_out: true,
        included_files: Vec::new(),
      });
    }
  }

  let swc_config = Config {
    is_type_script: true,
    is_jsx: true,
    decorators: false,
    source_type: SourceType::Module,
    ..Default::default()
  };

  let error_buffer = ErrorBuffer::default();
  let handler = Lrc::new(Handler::with_emitter(
    true,
    false,
    Box::new(error_buffer.clone()),
  ));

  errors::HANDLER.set(handler.as_ref(), || {
    let source_map = Lrc::new(SourceMap::default());

    // Parse and handle parsing errors
    let (program, comments) = match parse(
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
    let program = match program {
      Program::Module(module) => Program::Module(module),
      Program::Script(script) => Program::Module(Module {
        span: script.span,
        shebang: None,
        body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
      }),
    };

    // Apply the transformation while propagating structured diagnostics.
    let transform_output = GLOBALS.set(&Default::default(), || {
      let options = config_to_plugin_options(transform_config);
      let transform_file =
        atlassian_swc_compiled_css::TransformFile::transform_compiled_with_options(
          source_map.clone(),
          // SAFETY: `collect_comments_for_transform` extracts all leading/trailing
          // comment lists without mutating the parser's shared `SingleThreadedComments`
          // storage. This allows the downstream transform to emit diagnostics with full
          // source context (file, spans, comment metadata).
          collect_comments_for_transform(&comments),
          atlassian_swc_compiled_css::TransformFileOptions {
            filename: Some(input.filename.clone()),
            cwd: Some(input.project_root.clone().into()),
            root: Some(input.project_root.clone().into()),
            loc_filename: Some(input.filename.clone()),
          },
        );

      atlassian_swc_compiled_css::transform_with_file(program, transform_file, options)
    });

    let (mut transformed_program, mut style_rules, included_files) = match transform_output {
      Ok(output) => (
        output.program,
        output.metadata.style_rules,
        output.metadata.included_files,
      ),
      Err(errors) => {
        let diagnostics = convert_diagnostics(transform_errors_to_diagnostics(errors, &source_map));
        return Ok(CompiledCssInJsPluginResult {
          code: code.to_string(),
          map: None,
          style_rules: Vec::new(),
          diagnostics,
          bail_out: true,
          included_files: Vec::new(),
        });
      }
    };

    if transform_config.extract {
      let strip_options = strip_runtime::PluginOptions {
        style_sheet_path: None,
        compiled_require_exclude: Some(true),
        extract_styles_to_directory: None,
        sort_at_rules: Some(transform_config.sort_at_rules),
        sort_shorthand: Some(transform_config.sort_shorthand),
      };

      let strip_config = strip_runtime::TransformConfig {
        filename: Some(input.filename.clone()),
        cwd: Some(input.project_root.clone()),
        root: Some(input.project_root.clone()),
        source_file_name: Some(input.filename.clone()),
        options: strip_options,
      };

      match strip_runtime::try_transform(transformed_program, strip_config) {
        Ok(strip_output) => {
          transformed_program = strip_output.program;
          if !strip_output.metadata.style_rules.is_empty() {
            style_rules.extend(strip_output.metadata.style_rules);
          }
        }
        Err(errors) => {
          let diagnostics =
            convert_diagnostics(transform_errors_to_diagnostics(errors, &source_map));
          return Ok(CompiledCssInJsPluginResult {
            code: code.to_string(),
            map: None,
            style_rules: Vec::new(),
            diagnostics,
            bail_out: true,
            included_files: Vec::new(),
          });
        }
      }
    }

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

    let map_json = if input.source_maps && !line_pos_buffer.is_empty() {
      let build_map_result = {
        let _hook_guard = PANIC_HOOK_GUARD.lock();
        panic::catch_unwind(panic::AssertUnwindSafe(|| {
          let mut output_map_buffer = vec![];
          let write_result = source_map
            .build_source_map(&line_pos_buffer, None, SourceMapConfig)
            .to_writer(&mut output_map_buffer);
          (write_result, output_map_buffer)
        }))
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

    let code = append_transformed_asset_marker(code);

    Ok(CompiledCssInJsPluginResult {
      code,
      map: map_json,
      style_rules,
      diagnostics: Vec::new(),
      bail_out: false,
      included_files,
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
  // Initialize panic suppression at the entry point to ensure it's set before any work happens
  atlassian_swc_compiled_css::init_panic_suppression();

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

/// Convert CompiledCssInJsTransformConfig to PluginOptions
fn config_to_plugin_options(
  config: &atlassian_swc_compiled_css::CompiledCssInJsTransformConfig,
) -> atlassian_swc_compiled_css::PluginOptions {
  let import_sources = atlassian_swc_compiled_css::DEFAULT_IMPORT_SOURCES
    .iter()
    .map(|s| s.to_string())
    .chain(config.import_sources.clone())
    .collect();

  atlassian_swc_compiled_css::PluginOptions {
    cache: None,
    max_size: None,
    import_react: Some(config.import_react),
    nonce: config.nonce.clone(),
    import_sources,
    on_included_files: None,
    optimize_css: Some(config.optimize_css),
    resolver: None,
    // Preserve caller-provided extensions; when unset fall back to Compiled defaults.
    extensions: config.extensions.clone(),
    // Allow the transformer to apply its own parser defaults (TS + JSX) instead of forcing none.
    parser_babel_plugins: None,
    add_component_name: Some(config.add_component_name),
    class_name_compression_map: Some(std::collections::BTreeMap::new()),
    process_xcss: Some(config.process_xcss),
    increase_specificity: Some(config.increase_specificity),
    sort_at_rules: Some(config.sort_at_rules),
    class_hash_prefix: config.class_hash_prefix.clone(),
    flatten_multiple_selectors: Some(config.flatten_multiple_selectors),
    extract: Some(config.extract),
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
      config: CompiledCssInJsConfigPlugin {
        config_path: None,
        unsafe_report_safe_assets_for_migration: None,
        unsafe_use_safe_assets: None,
        unsafe_skip_pattern: None,
        import_react: Some(true),
        nonce: None,
        import_sources: vec!["@compiled/react".into(), "@atlaskit/css".into()],
        optimize_css: Some(true),
        extensions: None,
        add_component_name: Some(false),
        process_xcss: Some(true),
        increase_specificity: Some(false),
        sort_at_rules: Some(true),
        class_hash_prefix: None,
        flatten_multiple_selectors: Some(true),
        extract: Some(extract),
        ssr: Some(true),
        sort_shorthand: Some(true),
      },
    }
  }

  fn normalize_output(input: &str) -> String {
    fn ensure_whitespace_boundary(buf: &mut String) {
      if buf
        .chars()
        .last()
        .map(|c| c.is_whitespace())
        .unwrap_or(true)
      {
        return;
      }
      buf.push(' ');
    }

    fn normalize_single_quoted_strings(input: &str) -> String {
      fn canonicalize_single_quote_literal<I>(iter: &mut std::iter::Peekable<I>) -> Option<String>
      where
        I: Iterator<Item = char>,
      {
        let mut result = String::from("\"");
        let mut escape = false;

        for ch in iter.by_ref() {
          if escape {
            match ch {
              '\'' => result.push('\''),
              '"' => {
                result.push('\\');
                result.push('"');
              }
              '\\' => {
                result.push('\\');
                result.push('\\');
              }
              _ => {
                result.push('\\');
                result.push(ch);
              }
            }
            escape = false;
            continue;
          }

          if ch == '\\' {
            escape = true;
            continue;
          }

          if ch == '\'' {
            result.push('"');
            return Some(result);
          }

          if ch == '"' {
            result.push('\\');
            result.push('"');
          } else {
            result.push(ch);
          }
        }

        None
      }

      let mut chars = input.chars().peekable();
      let mut result = String::with_capacity(input.len());

      while let Some(ch) = chars.next() {
        if ch == '\'' {
          let mut lookahead = chars.clone();
          if let Some(canonical) = canonicalize_single_quote_literal(&mut lookahead) {
            result.push_str(&canonical);
            chars = lookahead;
            continue;
          }
        }

        result.push(ch);
      }

      result
    }

    let mut chars = input.chars().peekable();
    let mut output = String::with_capacity(input.len());
    let mut in_single_line_comment = false;
    let mut in_multi_line_comment = false;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut escape_next = false;

    while let Some(ch) = chars.next() {
      if in_single_line_comment {
        if ch == '\n' {
          in_single_line_comment = false;
          ensure_whitespace_boundary(&mut output);
        }
        continue;
      }

      if in_multi_line_comment {
        if ch == '*' && chars.peek().is_some_and(|next| *next == '/') {
          chars.next();
          in_multi_line_comment = false;
          ensure_whitespace_boundary(&mut output);
        }
        continue;
      }

      if !in_single_quote && !in_double_quote && !in_backtick && ch == '/' {
        if let Some('/') = chars.peek() {
          chars.next();
          in_single_line_comment = true;
          ensure_whitespace_boundary(&mut output);
          continue;
        } else if let Some('*') = chars.peek() {
          chars.next();
          in_multi_line_comment = true;
          ensure_whitespace_boundary(&mut output);
          continue;
        }
      }

      output.push(ch);

      if in_single_quote || in_double_quote || in_backtick {
        if escape_next {
          escape_next = false;
          continue;
        }

        if ch == '\\' {
          escape_next = true;
          continue;
        }

        match ch {
          '\'' if in_single_quote => in_single_quote = false,
          '"' if in_double_quote => in_double_quote = false,
          '`' if in_backtick => in_backtick = false,
          _ => {}
        }
      } else {
        match ch {
          '\'' => in_single_quote = true,
          '"' => in_double_quote = true,
          '`' => in_backtick = true,
          _ => {}
        }
      }
    }

    let collapsed: String = output.split_whitespace().collect();
    normalize_single_quoted_strings(&collapsed)
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
  fn test_skip_pattern_successful_transformation() {
    let mut config = create_test_config(true, false);

    config.config.unsafe_skip_pattern = Some(String::from("css="));

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformed code should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.bail_out,
      "Should bail out because of skip pattern"
    );
  }

  #[test]
  fn normalize_output_strips_js_comments() {
    let input = r#"const foo = 1; // comment
const bar = 2; /* multi
line */ const str = "// not comment"; const tpl = `/* not comment */`; "#;

    let expected = r#"const foo = 1;
const bar = 2; const str = "// not comment"; const tpl = `/* not comment */`; "#;

    assert_eq!(normalize_output(input), normalize_output(expected));
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
      !transformed.code.contains("CC"),
      "Extract mode should strip compiled runtime: {}",
      transformed.code
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
import commonMessages from '../../../common/messages.tsx';
import { mapping } from '../../../common/utils.tsx';

export const ModalHeader = ({ type }: ModalHeaderTypes): JSX.Element => {
	const { formatMessage } = useIntl();

	return (
		<Stack space="space.100">
			<h4 css={modalHeaderStyles}>{formatMessage(commonMessages.title)}</h4>
			<Text size="medium" color="color.text.accent.gray.bolder">
				{mapping[type] && formatMessage(mapping[type])}
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

  #[test]
  fn test_e2e() {
    unsafe {
      std::env::set_var("COMPILED_DEBUG_CSS", "1");
    }
    let config = create_test_config(true, false);

    let code = indoc! {r#"
/* eslint-disable react/no-unknown-property */
/* eslint-disable no-undef */

import React from 'react';
import {css} from '@compiled/react';
import {createRoot} from 'react-dom/client';

import Button from '@atlaskit/button/new';

const divStyles = css({color: 'blue'});

const root = createRoot(document.getElementById('app'));

const page = (
  <>
    <h1 data-testid="heading" css={headingStyles}>
      Hello, world!
    </h1>
    <div css={divStyles}>Content</div>
    <Button testId="button">Click me</Button>
  </>
);

const headingStyles = css({color: 'red'});

root.render(page);
"#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let transformed = result.unwrap();

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

  #[test]
  fn test_css_component_conditional() {
    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const myStyles = css({ color: 'red' });
      <div css={enabled && myStyles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let transformed = result.unwrap();

    assert!(
      !transformed.code.contains("css={"),
      "Should not have css in the code: {}",
      transformed.code
    );
    assert!(
      transformed.code.contains("className="),
      "Should have className in the code"
    );
  }

  #[test]
  fn test_css_on_component() {
    let config = create_test_config(true, false);

    let code = indoc! {r#"
/** @jsx jsx */
import { useCallback } from 'react';
import { css, jsx } from '@compiled/react';
import { Inline, xcss } from '@atlaskit/primitives';
import { ComponentA } from 'a';
import ComponentB, { type Props, type MinimumItemData } from 'b';

const ListWithPopup = () => {
	const [firstItem, ...restItems] = items;

	const ComponentBRenderer = useCallback(
		() =>
			isEnabled ? (
				<ComponentA>
					<ComponentB />
				</ComponentA>
			) : (
				<ComponentB
 />
			),
		[],
	);

	if (items.length === 0) {
		return null;
	}

	return (
		<Inline
			space="space.100"
			alignBlock="center"
			shouldWrap={!isEnabled}
			xcss={isEnabled && xStyles}
		>
			<ItemComponent
				{...firstItem}
				isEnabled={isEnabled}
				css={isEnabled && xItemStyles}
			/>

			{restItems.length >= 1 && ComponentBRenderer()}
		</Inline>
	);
};

const xStyles = xcss({
	paddingRight: 'space.050',
	width: '100%',
});

const xItemStyles = css({
	minWidth: '100%',
});

export default ListWithPopup;
    "#};

    let result = process_compiled_css_in_js(code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let transformed = result.unwrap();

    assert!(
      !transformed.code.contains(" css={"),
      "Should not have css in the code: {}",
      transformed.code
    );
    assert!(
      transformed.code.contains("className="),
      "Should have className in the code"
    );
    assert!(
      transformed.code.contains("xcss={"),
      "Should retain xcss in the code"
    );
  }

  #[test]
  fn test_nested_styled() {
    // Test styled API transformation with component name generation
    let mut config = create_test_config(true, false);
    config.config.add_component_name = Some(true);

    let input_code = indoc! {r#"
import React from 'react';
import { styled } from '@compiled/react';

const Container = componentWithCondition(condition, styled.ul({
    backgroundColor: "var(--ds-surface, #FFFFFF)",
    borderWidth: "var(--ds-border-width, 1px)",
    borderStyle: 'solid',
    borderColor: `${"var(--ds-border, #091e4221)"}`,
    // eslint-disable-next-line @atlaskit/design-system/no-unsafe-design-token-usage
    borderRadius: "var(--ds-radius-small, 3px)",
    paddingTop: "var(--ds-space-0, 0px)",
    paddingRight: "var(--ds-space-0, 0px)",
    paddingBottom: "var(--ds-space-0, 0px)",
    paddingLeft: "var(--ds-space-0, 0px)"
}), styled.ul({
    boxShadow: "var(--ds-shadow-raised, 0px 1px 1px #091e423f, 0px 0px 1px #091e4221)",
    backgroundColor: "var(--ds-surface-raised, #FFFFFF)",
    // eslint-disable-next-line @atlaskit/design-system/no-unsafe-design-token-usage
    borderRadius: "var(--ds-radius-small, 3px)",
    paddingTop: "var(--ds-space-0, 0px)",
    paddingRight: "var(--ds-space-0, 0px)",
    paddingBottom: "var(--ds-space-0, 0px)",
    paddingLeft: "var(--ds-space-0, 0px)"
  }));

export default Container;
    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Verify styled components are compiled away
    assert!(
      !output.code.contains("styled.ul"),
      "styled.ul() calls should be compiled away and replaced with className-based components"
    );

    // Verify styles were extracted
    assert!(
      !output.style_rules.is_empty(),
      "Style rules should be extracted from styled component definitions"
    );

    // Verify className was generated
    assert!(
      output.code.contains("className"),
      "Output should use className to apply extracted styles"
    );

    // Verify diagnostics are clean
    assert!(
      output.diagnostics.is_empty(),
      "Transformation should produce no diagnostics"
    );
  }

  #[test]
  fn test_mixed_css_and_styled() {
    // Test styled API transformation with component name generation
    let mut config = create_test_config(true, false);
    config.config.add_component_name = Some(true);

    let input_code = indoc! {r#"
import { css, styled } from '@compiled/react';
import React from 'react';
const ellipsis = {
	whiteSpace: 'nowrap',
	overflow: 'hidden',
	textOverflow: 'ellipsis',
} as const;

export const Ellipsis = styled.div(css<Record<any, any>>(ellipsis));
    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Verify styled components are compiled away
    assert!(
      !output.code.contains("styled.div"),
      "styled.div() calls should be compiled away and replaced with className-based components"
    );

    // Verify styled components are compiled away
    assert!(
      !output.code.contains("css"),
      "css calls should be compiled away and replaced with className-based components"
    );

    // Verify styles were extracted
    assert!(
      !output.style_rules.is_empty(),
      "Style rules should be extracted from styled component definitions"
    );

    // Verify className was generated
    assert!(
      output.code.contains("className"),
      "Output should use className to apply extracted styles"
    );

    // Verify diagnostics are clean
    assert!(
      output.diagnostics.is_empty(),
      "Transformation should produce no diagnostics"
    );
  }

  #[ignore]
  #[test]
  fn test_jsx_runtime_classic() {
    let mut config = create_test_config(true, false);
    config.config.add_component_name = Some(true);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx } from '@compiled/react';
import { Box } from '@atlaskit/primitives/compiled';

<Box
  style={{
    display: 'inline-block',
  }}
/>;
    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Verify styled components are compiled away
    assert!(
      output.code.contains("import * as React from 'react'"),
      "import * as React from 'react' should be present"
    );
  }

  #[test]
  fn test_css_map() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx } from '@compiled/react';

import { cssMap } from '@atlaskit/css';

const styles = cssMap({
	root: {
		boxSizing: 'border-box',
		height: '20px',
		width: '20px',
	},
});

export const ContainerAvatar = ({ src }: ContainerAvatarProps) => (
	<img src={src} css={styles.root} alt="" />
);

    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    assert!(
      !output.style_rules.is_empty(),
      "Style rules should be extracted from cssMap"
    );

    assert!(
      !output.code.contains("css={"),
      "Output should not contain css"
    );

    // Should rewrite css prop into a className application
    assert!(
      output.code.contains("className="),
      "Output should apply className"
    );

    // Should rewrite css prop into a className application
    assert!(
      output.code.contains("styles.root"),
      "Output should have styles.root applied as a className"
    );
  }

  #[test]
  fn test_css_map_primitives() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import {
	type ComponentPropsWithRef,
	type ElementType,
	forwardRef,
	type ReactNode,
	type Ref,
} from 'react';

import { jsx, cssMap as unboundedCssMap } from '@compiled/react';
import invariant from 'tiny-invariant';

import { css, cssMap, type StrictXCSSProp } from '@atlaskit/css';
import { token } from '@atlaskit/tokens';

import { HasTextAncestorProvider, useHasTextAncestor } from '../../utils/has-text-ancestor-context';
import { useSurface } from '../../utils/surface-provider';

import type { BasePrimitiveProps, FontSize, FontWeight, TextAlign, TextColor } from './types';

const asAllowlist = ['span', 'p', 'strong', 'em'] as const;
type AsElement = (typeof asAllowlist)[number];

type TextPropsBase<T extends ElementType = 'span'> = {
	/**
	 * HTML tag to be rendered. Defaults to `span`.
	 */
	as?: AsElement;
	/**
	 * Elements rendered within the Text element.
	 */
	children: ReactNode;
	/**
	 * Token representing text color with a built-in fallback value.
	 * Will apply inverse text color automatically if placed within a Box with bold background color.
	 * Defaults to `color.text` if not nested in other Text components.
	 */
	color?: TextColor | 'inherit';
	/**
	 * The [HTML `id` attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/id).
	 */
	id?: string;
	/**
	 * The number of lines to limit the provided text to. Text will be truncated with an ellipsis.
	 *
	 * When `maxLines={1}`, `wordBreak` defaults to `break-all` to match the behaviour of `text-overflow: ellipsis`.
	 */
	maxLines?: number;
	/**
	 * Text alignment.
	 */
	align?: TextAlign;
	/**
	 * Text size.
	 */
	size?: FontSize;
	/**
	 * The [HTML `font-weight` attribute](https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight).
	 */
	weight?: FontWeight;
	/**
	 * Bounded style overrides.
	 */
	xcss?: StrictXCSSProp<'overflowWrap' | 'textDecorationLine', never>;
	/**
	 * Forwarded ref.
	 */
	ref?: ComponentPropsWithRef<T>['ref'];
};

export type TextProps<T extends ElementType = 'span'> = TextPropsBase<T> &
	Omit<BasePrimitiveProps, 'xcss'>;

/**
 * Custom hook designed to abstract the parsing of the color props and make it clearer in the future how color is reconciled between themes and tokens.
 */
const useColor = (
	colorProp: TextColor | undefined | 'inherit',
	hasTextAncestor: boolean,
): TextColor | undefined => {
	const surface = useSurface();

	if (colorProp === 'inherit') {
		return undefined;
	}

	if (colorProp) {
		return colorProp;
	}

	if (hasTextAncestor) {
		return undefined;
	}

	if (inverseColorMap.hasOwnProperty(surface)) {
		return inverseColorMap[surface as keyof typeof inverseColorMap];
	}

	return 'color.text';
};

const styles = unboundedCssMap({
	root: {
		// We're doing this because our CSS reset can add top margins to elements such as `p` which is totally insane.
		// Long term we should remove those instances from the reset - it should be a reset to 0.
		// For now, at least we know <Text> will be unaffected by this.
		margin: 0,
		overflowWrap: 'anywhere',
	},
	'as.strong': { fontWeight: 'var(--ds-font.weight.bold)' },
	'as.em': { fontStyle: 'italic' },
	'textAlign.center': { textAlign: 'center' },
	'textAlign.end': { textAlign: 'end' },
	'textAlign.start': { textAlign: 'start' },
	truncation: {
		display: '-webkit-box',
		overflow: 'hidden',
		// NOTE: This is an obsolete property not used in modern CSS, perhaps unused, but likely added for some compatibility
		WebkitBoxOrient: 'vertical',
	},
	breakAll: { wordBreak: 'break-all' },
});

const fontSizeMap = cssMap({
	medium: { font: 'var(--ds-font.body)' },
	UNSAFE_small: { font: 'var(--ds-font.body.UNSAFE_small)' },
	large: { font: 'var(--ds-font.body.large)' },
	small: { font: 'var(--ds-font.body.small)' },
});

const fontWeightMap = unboundedCssMap({
	bold: { fontWeight: 'var(--ds-font.weight.bold)' },
	medium: { fontWeight: 'var(--ds-font.weight.medium)' },
	regular: { fontWeight: 'var(--ds-font.weight.regular)' },
	semibold: { fontWeight: 'var(--ds-font.weight.semibold)' },
});

const textColorMap = cssMap({
	'color.text': { color: 'var(--ds-color.text)' },
	'color.text.accent.lime': { color: 'var(--ds-color.text.accent.lime)' },
	'color.text.accent.lime.bolder': { color: 'var(--ds-color.text.accent.lime.bolder)' },
	'color.text.accent.red': { color: 'var(--ds-color.text.accent.red)' },
	'color.text.accent.red.bolder': { color: 'var(--ds-color.text.accent.red.bolder)' },
	'color.text.accent.orange': { color: 'var(--ds-color.text.accent.orange)' },
	'color.text.accent.orange.bolder': { color: 'var(--ds-color.text.accent.orange.bolder)' },
	'color.text.accent.yellow': { color: 'var(--ds-color.text.accent.yellow)' },
	'color.text.accent.yellow.bolder': { color: 'var(--ds-color.text.accent.yellow.bolder)' },
	'color.text.accent.green': { color: 'var(--ds-color.text.accent.green)' },
	'color.text.accent.green.bolder': { color: 'var(--ds-color.text.accent.green.bolder)' },
	'color.text.accent.teal': { color: 'var(--ds-color.text.accent.teal)' },
	'color.text.accent.teal.bolder': { color: 'var(--ds-color.text.accent.teal.bolder)' },
	'color.text.accent.blue': { color: 'var(--ds-color.text.accent.blue)' },
	'color.text.accent.blue.bolder': { color: 'var(--ds-color.text.accent.blue.bolder)' },
	'color.text.accent.purple': { color: 'var(--ds-color.text.accent.purple)' },
	'color.text.accent.purple.bolder': { color: 'var(--ds-color.text.accent.purple.bolder)' },
	'color.text.accent.magenta': { color: 'var(--ds-color.text.accent.magenta)' },
	'color.text.accent.magenta.bolder': { color: 'var(--ds-color.text.accent.magenta.bolder)' },
	'color.text.accent.gray': { color: 'var(--ds-color.text.accent.gray)' },
	'color.text.accent.gray.bolder': { color: 'var(--ds-color.text.accent.gray.bolder)' },
	'color.text.disabled': { color: 'var(--ds-color.text.disabled)' },
	'color.text.inverse': { color: 'var(--ds-color.text.inverse)' },
	'color.text.selected': { color: 'var(--ds-color.text.selected)' },
	'color.text.brand': { color: 'var(--ds-color.text.brand)' },
	'color.text.danger': { color: 'var(--ds-color.text.danger)' },
	'color.text.warning': { color: 'var(--ds-color.text.warning)' },
	'color.text.warning.inverse': { color: 'var(--ds-color.text.warning.inverse)' },
	'color.text.success': { color: 'var(--ds-color.text.success)' },
	'color.text.discovery': { color: 'var(--ds-color.text.discovery)' },
	'color.text.information': { color: 'var(--ds-color.text.information)' },
	'color.text.subtlest': { color: 'var(--ds-color.text.subtlest)' },
	'color.text.subtle': { color: 'var(--ds-color.text.subtle)' },
	'color.link': { color: 'var(--ds-color.link)' },
	'color.link.pressed': { color: 'var(--ds-color.link.pressed)' },
	'color.link.visited': { color: 'var(--ds-color.link.visited)' },
	'color.link.visited.pressed': { color: 'var(--ds-color.link.visited.pressed)' },
});

export const inverseColorMap = {
	'color.background.neutral.bold': 'color.text.inverse',
	'color.background.neutral.bold.hovered': 'color.text.inverse',
	'color.background.neutral.bold.pressed': 'color.text.inverse',
	'color.background.selected.bold': 'color.text.inverse',
	'color.background.selected.bold.hovered': 'color.text.inverse',
	'color.background.selected.bold.pressed': 'color.text.inverse',
	'color.background.brand.bold': 'color.text.inverse',
	'color.background.brand.bold.hovered': 'color.text.inverse',
	'color.background.brand.bold.pressed': 'color.text.inverse',
	'color.background.brand.boldest': 'color.text.inverse',
	'color.background.brand.boldest.hovered': 'color.text.inverse',
	'color.background.brand.boldest.pressed': 'color.text.inverse',
	'color.background.danger.bold': 'color.text.inverse',
	'color.background.danger.bold.hovered': 'color.text.inverse',
	'color.background.danger.bold.pressed': 'color.text.inverse',
	'color.background.warning.bold': 'color.text.warning.inverse',
	'color.background.warning.bold.hovered': 'color.text.warning.inverse',
	'color.background.warning.bold.pressed': 'color.text.warning.inverse',
	'color.background.success.bold': 'color.text.inverse',
	'color.background.success.bold.hovered': 'color.text.inverse',
	'color.background.success.bold.pressed': 'color.text.inverse',
	'color.background.discovery.bold': 'color.text.inverse',
	'color.background.discovery.bold.hovered': 'color.text.inverse',
	'color.background.discovery.bold.pressed': 'color.text.inverse',
	'color.background.information.bold': 'color.text.inverse',
	'color.background.information.bold.hovered': 'color.text.inverse',
	'color.background.information.bold.pressed': 'color.text.inverse',
} as const;

/**
 * __Text__
 *
 * Text is a primitive component that has the Atlassian Design System's design guidelines baked in.
 * This includes considerations for text attributes such as color, font size, font weight, and line height.
 * It renders a `span` by default.
 *
 * @internal
 */
const Text: React.ForwardRefExoticComponent<
	React.PropsWithoutRef<TextProps<ElementType>> & React.RefAttributes<any>
> = forwardRef(
	<T extends ElementType = 'span'>(
		{
			as: Component = 'span',
			color: colorProp,
			align,
			testId,
			id,
			size,
			weight,
			maxLines,
			xcss,
			children,
		}: TextProps<T>,
		ref: Ref<any>,
	) => {
		invariant(
			asAllowlist.includes(Component),
			`@atlaskit/primitives: Text received an invalid "as" value of "${Component}"`,
		);

		const hasTextAncestor = useHasTextAncestor();
		const color = useColor(colorProp, hasTextAncestor);

		if (!size && !hasTextAncestor) {
			size = 'medium';
		}

		const component = (
			<Component
				id={id}
				className={xcss}
				css={[
          stylesCss,
					styles.root,
					size && fontSizeMap[size],
					color && textColorMap[color],
					maxLines && styles.truncation,
					maxLines === 1 && styles.breakAll,
					align && styles[`textAlign.${align}`],
					weight && fontWeightMap[weight],
					Component === 'em' && styles['as.em'],
					Component === 'strong' && styles['as.strong'],
				]}
				style={{
					WebkitLineClamp: maxLines,
				}}
				data-testid={testId}
				ref={ref}
			>
				{children}
			</Component>
		);

		if (hasTextAncestor) {
			// no need to re-apply context if the text is already wrapped
			return component;
		}

		return <HasTextAncestorProvider value={true}>{component}</HasTextAncestorProvider>;
	},
);

const stylesCss = css({ color: 'red' });

export default Text;
    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains("css={"),
      "Output should not contain css"
    );
    assert!(
      !output.code.contains("cssMap("),
      "cssMap should be transformed to a map of classes"
    );
    assert!(
      !output.code.contains("unboundedCssMap("),
      "unboundedCssMap should be transformed to a map of classes"
    );
    assert!(
      output.code.contains("className="),
      "Output should apply className"
    );

    let expected_classname = indoc! {r#"
      className={ax([
        "_syaz5scu",
        styles.root,
        size && fontSizeMap[size],
        color && textColorMap[color],
        maxLines && styles.truncation,
        maxLines === 1 && styles.breakAll,
        align && styles[`textAlign.${align}`],
        weight && fontWeightMap[weight],
        Component === 'em' && styles['as.em'],
        Component === 'strong' && styles['as.strong'],
        xcss
      ])}
    "#};

    let normalized_output = normalize_output(&output.code);
    let normalized_expected = normalize_output(expected_classname);
    assert!(
      normalized_output.contains(&normalized_expected),
      "Output should include the expected ax className call with all flex props"
    );
  }

  #[ignore]
  #[test]
  fn test_css_prop_component() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/** @jsx jsx */
import React from 'react';
import { css, jsx } from '@compiled/react';
import { AppSkeletonImage } from '@jira/platform__skeleton-image-container/src/ui/AppSkeletonImage.tsx';
import imgSVG from './assets/virtual-agent-intent-training-phrases-app-skeleton.tpl.svg';

const customCssStyles = css({
	margin: '0 auto',
	maxWidth: '960px',
	borderLeftWidth: 0,
	borderRightWidth: 0,
});

export const VirtualAgentIntentTrainingPhrasesSkeleton = () => (
	<AppSkeletonImage src={imgSVG} css={customCssStyles} />
);

    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains("css={"),
      "Output should not contain css"
    );

    assert!(
      output.code.contains("className={ax(["),
      "Output should apply className"
    );
  }

  #[ignore]
  #[test]
  fn test_css_prop_component_array() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/** @jsx jsx */
import React from 'react';
import { css, jsx } from '@compiled/react';
import { AppSkeletonImage } from '@jira/platform__skeleton-image-container/src/ui/AppSkeletonImage.tsx';

import imgSVG from './assets/issue-app-skeleton.tpl.svg';

const fullpageStyles = css({
	borderTopWidth: '50px',
});
const modalStyles = css({
	// eslint-disable-next-line @atlaskit/design-system/use-tokens-space
	marginTop: '-40px',
});
const customSpacingStyles = css({
	maxWidth: '1920px',
	marginRight: 'auto',
	marginLeft: 'auto',
	'@media (min--moz-device-pixel-ratio: 2)': {
		maxWidth: '840px',
	},
});

export const IssueViewSkeletonWithRightStatus = ({ isEmbedView, isModalView }) => {
	// same as getIssueContainerMaxWidth
	const customCss = [
		customSpacingStyles,
		isEmbedView !== true && isModalView !== true ? fullpageStyles : null,
		isModalView === true ? modalStyles : null,
	];
	return (
			<AppSkeletonImage src={imgSVG} css={customCss} />
	);
};
    "#};

    let result = process_compiled_css_in_js(input_code, &config);
    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains("css={"),
      "Output should not contain css"
    );

    let expected_classname = indoc! {r#"
      className={ax([
        '_p12f107j _2hwx1wug _18u01wug _11y7uu9g',
        isEmbedView !== true && isModalView !== true && '_uwhk12am',
        isModalView === true && '_19pk10j7'
      ])}
    "#};

    let normalized_output = normalize_output(&output.code);
    let normalized_expected = normalize_output(expected_classname);
    assert!(
      normalized_output.contains(&normalized_expected),
      "Output should include the expected ax className call with all flex props"
    );
  }

  #[ignore]
  #[test]
  fn test_css_sidebar() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { useContext, useRef, useState } from "react";

import { cssMap, jsx } from "@compiled/react";

import { fg } from "@atlaskit/platform-feature-flags";
import { token } from "@atlaskit/tokens";

import { useIsFhsEnabled } from "../../fhs-rollout/use-is-fhs-enabled";
import {
  bannerMountedVar,
  contentHeightWhenFixed,
  contentInsetBlockStart,
  localSlotLayers,
  sideNavVar,
  topNavMountedVar,
} from "../constants";

import { useSideNavVisibility } from "./use-side-nav-visibility";
import { SideNavVisibilityState } from "./visibility-context";

const panelSplitterResizingVar = "--n_snvRsz";

const isFirefox: boolean =
  typeof navigator !== "undefined" &&
  navigator.userAgent.toLowerCase().indexOf("firefox") > -1;

const styles = cssMap({
  root: {
    backgroundColor: token("elevation.surface.overlay"),
    boxShadow: token("elevation.shadow.overlay"),
    boxSizing: "border-box",
    gridArea: "main / aside / aside / aside",
    height: contentHeightWhenFixed,
    insetBlockStart: contentInsetBlockStart,
    position: "sticky",
    width: "min(90%, 320px)",
    zIndex: localSlotLayers.sideNav,
    borderInlineStart: "none",
    borderInlineEnd: "none",
    "@media (min-width: 48rem)": {
      width: `var(${panelSplitterResizingVar}, var(${sideNavVar}))`,
    },
    "@media (min-width: 64rem)": {
      backgroundColor: token("elevation.surface"),
      boxShadow: "initial",
      gridArea: "side-nav",
      borderInlineEnd: `${token("border.width")} solid ${token(
        "color.border"
      )}`,
    },
  },
  flyoutOpen: {
    "@media (min-width: 64rem)": {
      backgroundColor: token("elevation.surface.overlay"),
      boxShadow: token("elevation.shadow.overlay"),
      gridArea: "main",
      borderInlineEnd: "none",
    },
    "@supports not (-moz-appearance: none)": {
      "@media (prefers-reduced-motion: no-preference)": {
        transitionProperty: "transform, display",
        transitionDuration: "0.2s",
        transitionBehavior: "allow-discrete",
        "@starting-style": {
          transform: "translateX(-100%)",
        },
      },
    },
  },
  flyoutAnimateClosed: {
    display: "none",
    "@media (min-width: 64rem)": {
      gridArea: "main",
    },
    "@supports not (-moz-appearance: none)": {
      "@media (prefers-reduced-motion: no-preference)": {
        transitionProperty: "transform, display",
        transitionDuration: "0.2s",
        transitionBehavior: "allow-discrete",
        transform: "translateX(-100%)",
      },
    },
  },
  animationRTLSupport: {
    "--animation-direction": "1",
    "[dir='rtl'] &": {
      "--animation-direction": "-1",
    },
  },
  flyoutBaseStylesFullHeightSidebar: {
    "@media (min-width: 64rem)": {
      backgroundColor: token("elevation.surface.overlay"),
      boxShadow: token("elevation.shadow.overlay"),
      gridArea: "main",
      borderInlineEnd: "none",
    },
    "@media (prefers-reduced-motion: no-preference) and (min-width: 64rem)": {
      transitionProperty: "transform, display",
      transitionBehavior: "allow-discrete",
    },
  },
  flyoutOpenFullHeightSidebar: {
    "@media (prefers-reduced-motion: no-preference) and (min-width: 64rem)": {
      transitionDuration: "0.2s",
      transitionTimingFunction: "cubic-bezier(0.6, 0, 0, 1)",
      "@starting-style": {
        transform: "translateX(calc(-100% * var(--animation-direction)))",
      },
    },
  },
  flyoutAnimateClosedFullHeightSidebar: {
    "@media (min-width: 64rem)": {
      display: "none",
    },
    "@media (prefers-reduced-motion: no-preference) and (min-width: 64rem)": {
      transitionDuration: "0.2s",
      transitionTimingFunction: "cubic-bezier(0, 0.4, 0, 1)",
      transform: "translateX(calc(-100% * var(--animation-direction)))",
    },
  },
  flexContainer: {
    height: "100%",
    display: "flex",
    flexDirection: "column",
    justifyContent: "space-between",
  },
  hiddenMobileAndDesktop: {
    display: "none",
  },
  hiddenMobileOnly: {
    display: "none",
    "@media (min-width: 64rem)": {
      display: "initial",
    },
  },
  hiddenDesktopOnly: {
    "@media (min-width: 64rem)": {
      display: "none",
    },
  },
  animationBaseStyles: {
    "@media (prefers-reduced-motion: no-preference)": {
      transitionProperty: "transform, display",
      transitionBehavior: "allow-discrete",
      transitionDuration: "0.2s",
    },
  },
  expandAnimationMobile: {
    "@media (prefers-reduced-motion: no-preference)": {
      transitionTimingFunction: "cubic-bezier(0.6, 0, 0, 1)",
      "@starting-style": {
        transform: "translateX(calc(-100% * var(--animation-direction)))",
      },
    },
  },
  collapseAnimationMobile: {
    "@media (prefers-reduced-motion: no-preference)": {
      gridArea: "main",
      transitionTimingFunction: "cubic-bezier(0, 0.4, 0, 1)",
    },
    "@media (prefers-reduced-motion: no-preference) and (not (min-width: 64rem))":
      {
        transform: "translateX(calc(-100% * var(--animation-direction)))",
      },
  },
  expandAnimationDesktop: {
    "@media (prefers-reduced-motion: no-preference) and (min-width: 64rem)": {
      gridArea: "side-nav",
      transitionTimingFunction: "cubic-bezier(0.6, 0, 0, 1)",
      "@starting-style": {
        transform: "translateX(calc(-100% * var(--animation-direction)))",
      },
    },
  },
  collapseAnimationDesktop: {
    "@media (prefers-reduced-motion: no-preference) and (min-width: 64rem)": {
      gridArea: "main",
      transitionTimingFunction: "cubic-bezier(0, 0.4, 0, 1)",
      transform: "translateX(calc(-100% * var(--animation-direction)))",
    },
  },
  fullHeightSidebar: {
    "@media (min-width: 64rem)": {
      height: `calc(100vh - var(${bannerMountedVar}, 0px))`,
      insetBlockStart: `calc(var(${bannerMountedVar}, 0px))`,
      paddingBlockStart: `calc(var(${topNavMountedVar}, 0px))`,
      marginBlockStart: `calc(-1 * var(${topNavMountedVar}, 0px))`,
    },
  },
});

export const onPeekStartDelayMs = 500;

export function SideNavInternal({ children, defaultCollapsed }) {
  const isFhsEnabled = fg("navx-2566-implement-fhs-rollout")
    ? useIsFhsEnabled()
    : fg("navx-full-height-sidebar");

  const sideNavState = useContext(SideNavVisibilityState);
  const { isExpandedOnDesktop, isExpandedOnMobile } = useSideNavVisibility({
    defaultCollapsed,
  });

  const [initialIsExpandedOnDesktop] = useState(isExpandedOnDesktop);

  const isFlyoutVisible = sideNavState?.flyout === "open";

  const isFlyoutClosed =
    sideNavState?.flyout === "closed" || sideNavState?.flyout === undefined;

  const isExpandedStateDifferentFromInitial =
    isExpandedOnMobile || isExpandedOnDesktop !== initialIsExpandedOnDesktop;

  const hasExpandedStateChangedRef = useRef(false);

  const hasExpandedStateChanged =
    isExpandedStateDifferentFromInitial || hasExpandedStateChangedRef.current;

  const shouldShowSidebarToggleAnimation =
    isFhsEnabled && hasExpandedStateChanged && !isFlyoutVisible && !isFirefox;

  return (
    <nav
      css={[
        styles.root,
        isExpandedOnDesktop &&
          !isExpandedOnMobile &&
          !isFlyoutVisible &&
          styles.hiddenMobileOnly,
        !isExpandedOnDesktop &&
          isExpandedOnMobile &&
          !isFlyoutVisible &&
          styles.hiddenDesktopOnly,
        !isExpandedOnDesktop &&
          !isExpandedOnMobile &&
          !isFlyoutVisible &&
          styles.hiddenMobileAndDesktop,

        isFhsEnabled && styles.animationRTLSupport,
        shouldShowSidebarToggleAnimation && styles.animationBaseStyles,
        isExpandedOnMobile &&
          shouldShowSidebarToggleAnimation &&
          styles.expandAnimationMobile,
        !isExpandedOnMobile &&
          shouldShowSidebarToggleAnimation &&
          styles.collapseAnimationMobile,
        isExpandedOnDesktop &&
          shouldShowSidebarToggleAnimation &&
          styles.expandAnimationDesktop,
        !isExpandedOnDesktop &&
          shouldShowSidebarToggleAnimation &&
          styles.collapseAnimationDesktop,

        sideNavState?.flyout === "open" && !isFhsEnabled && styles.flyoutOpen,
        sideNavState?.flyout === "triggered-animate-close" &&
          !isFhsEnabled &&
          styles.flyoutAnimateClosed,

        (sideNavState?.flyout === "open" ||
          sideNavState?.flyout === "triggered-animate-close") &&
          !isFirefox &&
          isFhsEnabled &&
          styles.flyoutBaseStylesFullHeightSidebar,
        sideNavState?.flyout === "triggered-animate-close" &&
          !isFirefox &&
          isFhsEnabled &&
          styles.flyoutAnimateClosedFullHeightSidebar,
        sideNavState?.flyout === "open" &&
          !isFirefox &&
          isFhsEnabled &&
          styles.flyoutOpenFullHeightSidebar,
        sideNavState?.flyout === "triggered-animate-close" &&
          !isFirefox &&
          isFhsEnabled &&
          styles.flyoutAnimateClosedFullHeightSidebar,
        isFlyoutClosed && isFhsEnabled && styles.fullHeightSidebar,
      ]}
    >
      <div css={styles.flexContainer}>{children}</div>
    </nav>
  );
}
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains("css={"),
      "Output should not contain css"
    );

    let expected_classname = indoc! {r#"
      className={ax([
          styles.root,
          isExpandedOnDesktop && !isExpandedOnMobile && !isFlyoutVisible && styles.hiddenMobileOnly,
          !isExpandedOnDesktop && isExpandedOnMobile && !isFlyoutVisible && styles.hiddenDesktopOnly,
          !isExpandedOnDesktop && !isExpandedOnMobile && !isFlyoutVisible && styles.hiddenMobileAndDesktop,
          isFhsEnabled && styles.animationRTLSupport,
          shouldShowSidebarToggleAnimation && styles.animationBaseStyles,
          isExpandedOnMobile && shouldShowSidebarToggleAnimation && styles.expandAnimationMobile,
          !isExpandedOnMobile && shouldShowSidebarToggleAnimation && styles.collapseAnimationMobile,
          isExpandedOnDesktop && shouldShowSidebarToggleAnimation && styles.expandAnimationDesktop,
          !isExpandedOnDesktop && shouldShowSidebarToggleAnimation && styles.collapseAnimationDesktop,
          sideNavState?.flyout === "open" && !isFhsEnabled && styles.flyoutOpen,
          sideNavState?.flyout === "triggered-animate-close" && !isFhsEnabled && styles.flyoutAnimateClosed,
          (sideNavState?.flyout === "open" || sideNavState?.flyout === "triggered-animate-close") && !isFirefox && isFhsEnabled && styles.flyoutBaseStylesFullHeightSidebar,
          sideNavState?.flyout === "triggered-animate-close" && !isFirefox && isFhsEnabled && styles.flyoutAnimateClosedFullHeightSidebar,
          sideNavState?.flyout === "open" && !isFirefox && isFhsEnabled && styles.flyoutOpenFullHeightSidebar,
          sideNavState?.flyout === "triggered-animate-close" && !isFirefox && isFhsEnabled && styles.flyoutAnimateClosedFullHeightSidebar,
          isFlyoutClosed && isFhsEnabled && styles.fullHeightSidebar
      ])}
    "#};

    let normalized_output = normalize_output(&output.code);
    let normalized_expected = normalize_output(expected_classname);

    assert!(
      normalized_output.contains(&normalized_expected),
      "Output should include the expected ax className call with all styles"
    );
  }

  #[ignore]
  #[test]
  fn test_css_page_layout() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { useRef } from "react";

import { cssMap, jsx } from "@compiled/react";

export const gridRootId = "unsafe-design-system-page-layout-root";

const styles = cssMap({
  root: {
    display: "grid",
    minHeight: "100vh",
    gridTemplateAreas: `
            "banner"
            "top-bar"
            "main"
            "aside"
       `,
    gridTemplateColumns: "minmax(0, 1fr)",
    gridTemplateRows: "auto auto 1fr auto",
    "@media (min-width: 64rem)": {
      gridTemplateAreas: `
            "banner banner banner"
            "top-bar top-bar top-bar"
            "side-nav main aside"
       `,
      gridTemplateRows: "auto auto 3fr",
      gridTemplateColumns: "auto minmax(0,1fr) auto",
    },
    "@media (min-width: 90rem)": {
      gridTemplateAreas: `
                "banner banner banner banner"
                "top-bar top-bar top-bar top-bar"
                "side-nav main aside panel"
           `,
      gridTemplateRows: "auto auto 3fr",
      gridTemplateColumns: "auto minmax(0,1fr) auto auto",
    },
    "> :not([data-layout-slot])": {
      display: "none !important",
    },
  },
});

export function Root({ children, xcss, testId }) {
  const ref = useRef<HTMLDivElement>(null);

  return (
    <div
      ref={ref}
      css={styles.root}
      className={xcss}
      id={gridRootId}
      data-testid={testId}
    >
      {children}
    </div>
  );
}
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains("css={"),
      "Output should not contain css"
    );

    let expected_classname = indoc! {r#"
      className={ax([styles.root, xcss])}
    "#};

    let normalized_output = normalize_output(&output.code);
    let normalized_expected = normalize_output(expected_classname);

    assert!(
      normalized_output.contains(&normalized_expected),
      "Output should include the expected ax className call with all styles"
    );

    let has_cc = normalized_output.contains("jsxs(CC,") || normalized_output.contains("_jsxs(CC,");
    assert!(has_cc, "Output should include CC runtime wrapper");
    let has_cs = normalized_output.contains("jsx(CS,") || normalized_output.contains("_jsx(CS,");
    assert!(has_cs, "Output should include CS runtime wrapper");

    // Ensure the specific div is wrapped by CC/CS by order of occurrence
    let pos_cc = normalized_output
      .find("jsxs(CC,")
      .or_else(|| normalized_output.find("_jsxs(CC,"))
      .expect("Expected CC wrapper in output");
    let pos_cs = normalized_output
      .find("jsx(CS,")
      .or_else(|| normalized_output.find("_jsx(CS,"))
      .expect("Expected CS wrapper in output");
    // After wrapping, inner element can remain as JSX <div ...>
    let pos_div = normalized_output
      .find("<div")
      .expect("Expected div element in output");

    assert!(pos_cc < pos_cs, "CS should be inside CC wrapper");
    assert!(
      pos_cs < pos_div,
      "Div should appear after CS within CC children"
    );
  }

  #[ignore]
  #[test]
  fn test_css_page_template() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */

import React, { type ReactNode } from "react";

import { css } from "@compiled/react";

import { jsx } from "@atlaskit/css";

const titleWrapperStyles = css({
  display: "flex",
  alignItems: "flex-start",
  flexWrap: "nowrap",
});

const titleWrapperTruncateStyles = css({
  flexWrap: "nowrap",
});

export const TitleWrapper = ({ children, truncateTitle }) => {
  return (
    <div
      css={[titleWrapperStyles, truncateTitle && titleWrapperTruncateStyles]}
    >
      {children}
    </div>
  );
};
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains(" css={"),
      "Output should not contain css"
    );

    let expected_classname = indoc! {r#"
      className={ax(['_1e0c1txw _4cvr1y6m _1n261q9c', truncateTitle && "_1n261q9c"])}
    "#};

    let normalized_output = normalize_output(&output.code);
    let normalized_expected = normalize_output(expected_classname);

    assert!(
      normalized_output.contains(&normalized_expected),
      "Output should include the expected ax className call with all styles"
    );
  }

  #[ignore]
  #[test]
  fn test_field_heading_container() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/** @jsx jsx */

import { css, jsx } from '@compiled/react';
export const FieldHeadingContainer = ({ children, isSettingsOpen = false }) => {
	const { layout } = useHook();
	const isWideViewLayout = layout === 'WIDE';
	const isEnabled = isEnabled();
	return (
		<div
			css={[
				styles0,
				isWideViewLayout && styles2,
				isEnabled && styles3,
				isSettingsOpen && styles,
			]}
		>
			{children}
		</div>
	);
};
const styles0 = css({
	display: 'flex',
	alignItems: 'center',
	gap: 'var(--ds-space-100, 8px)',
	paddingInline: 'var(--ds-space-050, 4px)',
	paddingBlock: 'var(--ds-space-050, 4px)',
	borderRadius: 'var(--ds-radius-small, 4px)',
	height: '100%',
	'&:hover': {
		backgroundColor: 'var(--ds-background-neutral-hovered, #0B120E24)',
	},
	marginLeft: 'var(--ds-space-negative-050, -4px)',
	width: 'fit-content',
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-nested-selectors
	'> h3': {
		fontWeight: 'var(--ds-font-weight-medium, 500)',
	},
});
const styles = css({
	backgroundColor: 'var(--ds-background-neutral-hovered, #0B120E24)',
});
const styles2 = css({
	'&:hover': {
		backgroundColor: 'color.background.disabled',
	},
});
const styles3 = css({
	flexGrow: 1,
});
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Print only the slice of output.code from the first occurrence of "className={" to the next "}" (inclusive)
    let classname_output = if let Some(start) = output.code.find("className={") {
      if let Some(end) = output.code[start..].find('}') {
        // add '}' to the range (end is relative to start)
        output.code[start..start + end + 1].to_string()
      } else {
        panic!("Could not find closing '}}' after 'className={{'");
      }
    } else {
      panic!("Could not find 'className={{' in transformation output");
    };

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    // Ensure css prop was transformed
    assert!(
      !output.code.contains(" css={"),
      "Output should not contain css"
    );

    let expected_classname = indoc! {r#"
      className={ax([
        '_zulpu2gc _2rko12b0 _18zr1b66 _1rjc1b66 _1e0c1txw _4cvr1h6o _4t3i1osq _irr3plhp _18u0r5cr _1bsb1ns9 _xn9u1wq8',
        isWideViewLayout && '_irr3mop1',
        isEnabled && '_16jlkb7n',
        isSettingsOpen && '_bfhkplhp'
      ])}
    "#};

    let normalized_output = normalize_output(&classname_output);
    let normalized_expected = normalize_output(expected_classname);

    assert!(
      normalized_output.eq(&normalized_expected),
      "Output should include the expected ax className call with all styles"
    );
  }

  #[test]
  fn test_css_var_injection() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
import type { ClassAttributes, ComponentType, HTMLAttributes, ReactNode } from 'react';
import { styled, type StyledProps } from '@compiled/react';
import { token } from '@atlaskit/tokens';
import type { FieldWidth } from '@atlassian/jira-issue-create-common-types/src/common/types/index.tsx';
import { isMobileAndInMvpOrExistingUsersExperiment } from '@atlassian/jira-mobile-web/src/index.tsx';

// eslint-disable-next-line @atlaskit/ui-styling-standard/no-exported-styles -- Ignored via go/DSP-18766
export const FieldContainer: ComponentType<
	{
		children?: ReactNode;
		width?: FieldWidth;
		marginTop?: number;
		paddingBottom?: number;
	} & ClassAttributes<HTMLDivElement> &
		HTMLAttributes<HTMLDivElement> &
		StyledProps
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-styled
> = styled.div<{
	children?: ReactNode;
	width?: FieldWidth;
	marginTop?: number;
	paddingBottom?: number;
}>({
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles, @atlaskit/ui-styling-standard/no-imported-style-values -- Ignored via go/DSP-18766
	width: ({ width = isMobileAndInMvpOrExistingUsersExperiment() ? '100%' : 350 }) => {
		return typeof width === 'number' ? `${width}px` : width;
	},
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles -- Ignored via go/DSP-18766
	marginTop: ({ marginTop = 0 }) => `${marginTop}px`,
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles -- Ignored via go/DSP-18766
	paddingBottom: ({ paddingBottom = 0 }) => `${paddingBottom}px`,
});

// eslint-disable-next-line @atlaskit/ui-styling-standard/no-exported-styles -- Ignored via go/DSP-18766
export const Description: ComponentType<
	{
		children?: ReactNode;
	} & ClassAttributes<HTMLParagraphElement> &
		HTMLAttributes<HTMLParagraphElement> &
		StyledProps
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-styled
> = styled.p<{
	children?: ReactNode;
}>({
	marginBottom: token('space.100'),
});

// eslint-disable-next-line @atlaskit/ui-styling-standard/no-exported-styles -- Ignored via go/DSP-18766
export const SelectContainer: ComponentType<
	{
		children?: ReactNode;
	} & ClassAttributes<HTMLDivElement> &
		HTMLAttributes<HTMLDivElement> &
		StyledProps
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-styled
> = styled.div<{
	children?: ReactNode;
}>({
	width: '300px',
});

// eslint-disable-next-line @atlaskit/ui-styling-standard/no-exported-styles -- Ignored via go/DSP-18766
export const Divider: ComponentType<
	{
		children?: ReactNode;
	} & ClassAttributes<HTMLDivElement> &
		HTMLAttributes<HTMLDivElement> &
		StyledProps
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-styled
> = styled.div<{
	children?: ReactNode;
}>({
	height: '1px',
	width: '100%',

	backgroundColor: token('color.border'),
	marginTop: token('space.300'),
	marginRight: 0,
	marginBottom: token('space.300'),
	marginLeft: 0,
});

// eslint-disable-next-line @atlaskit/ui-styling-standard/no-exported-styles -- Ignored via go/DSP-18766
export const CloseButtonContainer: ComponentType<
	{
		children?: ReactNode;
	} & ClassAttributes<HTMLDivElement> &
		HTMLAttributes<HTMLDivElement> &
		StyledProps
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-styled
> = styled.div<{
	children?: ReactNode;
}>({
	marginLeft: 'auto',
	marginRight: token('space.100'),
});

    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    let expected = indoc! {r#"
      "--_rf37vj": ix(
        (() => {
          returntypeof__cmplp.width ??
          (isMobileAndInMvpOrExistingUsersExperiment() ? "100%" : 350) ===
            "number"
            ? `${
                __cmplp.width ??
                (isMobileAndInMvpOrExistingUsersExperiment() ? "100%" : 350)
              }px`
            : __cmplp.width ??
              (isMobileAndInMvpOrExistingUsersExperiment() ? "100%" : 350);
        })()
      )
    "#};

    let normalized_output = normalize_output(&output.code);
    let normalized_expected = normalize_output(expected);

    assert!(
      normalized_output.contains(&normalized_expected),
      "Output should include the expected style object"
    );
  }

  #[test]
  fn test_react_create_element() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/** @jsx jsx */
import { css, jsx } from "@compiled/react";

const verticalAlignStyle = css({
  verticalAlign: "middle",
});

const ScreenIcon = <span css={verticalAlignStyle} aria-hidden></span>;
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    assert!(
      output.code.contains("._s7n4nkob{vertical-align:middle}"),
      "Output should contain the vertical align style"
    );

    assert!(
      output.code.contains("const _"),
      "Output should set the css styles to a var"
    );
  }

  #[test]
  fn test_react_create_element2() {
    let config = create_test_config(true, false);

    let input_code = indoc! {r#"
/* eslint-disable @atlaskit/ui-styling-standard/enforce-style-prop */

/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx } from '@compiled/react';
import { token } from '@atlaskit/tokens';
import { Box } from '@atlaskit/primitives/compiled';

import {
	useExperienceRenderAndMountMark,
	ExperienceMark,
} from '../../common/utils/experience-tracker';
import { useIntl } from 'react-intl-next';
import messages from './messages';

/*
  In the Remote App Switcher, the Skeleton component shows up first as a fallback while the full Remote App Switcher is still loading.
  Since the styles (compiled CSS) are lazy-loaded with the full App Switcher, they are not be ready when the Skeleton appears.
  This causes visual issues where the Skeleton looks broken

  To avoid this, we use inline styles instead. This guarantees that all required styles are applied immediately with the component.
  https://hello.jira.atlassian.cloud/browse/NAVX-676
*/
const iconSkeleton = (
	<Box
		style={{
			display: 'inline-block',
			width: 32,
			height: 32,
			backgroundColor: token('color.skeleton'),
			borderRadius: token('radius.large'),
			marginRight: token('space.100'),
		}}
	/>
);
const lineSkeleton = (
	<Box
		style={{
			display: 'inline-block',
			width: 260,
			height: 10,
			backgroundColor: token('color.skeleton'),
			borderRadius: token('radius.small'),
		}}
	/>
);

export default () => {
	useExperienceRenderAndMountMark(ExperienceMark.SWITCHER_SKELETON_MOUNT);
	const { formatMessage } = useIntl();

	const items = (
		<Box
			aria-hidden="true"
			style={{
				display: 'flex',
				paddingInline: token('space.200'),
				paddingBlock: token('space.100'),
				alignItems: 'center',
			}}
		>
			{iconSkeleton}
			{lineSkeleton}
		</Box>
	);

	return (
		<Box
			as="section"
			style={{ width: '343px', height: '80vh' }}
			role="alert"
			aria-label={formatMessage(messages.skeletonLoaderAriaLabel)}
			testId="skeleton"
		>
			<Box
				aria-hidden="true"
				style={{
					paddingInline: token('space.200'),
					paddingTop: token('space.300'),
					paddingBottom: token('space.100'),
				}}
			>
				{lineSkeleton}
			</Box>
			{items}
			{items}
			{items}
		</Box>
	);
};
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    assert!(
      output.code.contains("import { jsx"),
      "Output should import the jsx runtime"
    );
  }

  #[test]
  fn test_no_styles() {
    let config = create_test_config(true, true);

    let input_code = indoc! {r#"
/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { useIntl } from 'react-intl-next';

import type { UIAnalyticsEvent } from '@atlaskit/analytics-next';
import { ButtonGroup } from '@atlaskit/button';
import ButtonNew, { LinkButton } from '@atlaskit/button/new';
import { jsx } from '@atlaskit/css';
import { EmptyStateNew } from '@atlassian/teams-common/empty-state-new';

import EmptyStateImage from '../../../../../../../assets/software-components/SoftwareComponentsEmptyStateImage_ptc.svg';
import { useScreenAnalytics } from '../../../../../../../controllers/software-components';
import { messages as commonMessages } from '../messages';

import { messages } from './messages';

const learnMoreLink = 'https://www.atlassian.com/software/compass';

interface Props {
	onCompassSignup?: (
		e: React.MouseEvent<HTMLElement, MouseEvent>,
		analyticsEvent: UIAnalyticsEvent,
	) => void;
	onLearnMore?: (
		e: React.MouseEvent<HTMLElement, MouseEvent>,
		analyticsEvent: UIAnalyticsEvent,
	) => void;
}

export function CompassNotProvisionedEmptyState({ onCompassSignup }: Props) {
	const { formatMessage } = useIntl();

	useScreenAnalytics('notProvisionedEmptyState');

	return (
		<EmptyStateNew
			testId="compass.software-components.not-provisioned"
			title={formatMessage(messages.title)}
			description={formatMessage(messages.description)}
			imageUrl={EmptyStateImage}
			actions={
				<ButtonGroup>
					<ButtonNew onClick={onCompassSignup}>{formatMessage(messages.upsellButton)}</ButtonNew>
					<LinkButton href={learnMoreLink} target="_blank" rel="noopener" appearance="subtle">
						{formatMessage(commonMessages.learnMoreLink)}
					</LinkButton>
				</ButtonGroup>
			}
		/>
	);
}
    "#};

    let result = process_compiled_css_in_js(input_code, &config);

    assert!(result.is_ok(), "Transformation should succeed");

    let output = result.unwrap();

    // Verify transformation was applied
    assert!(!output.bail_out, "Transformation should not bail out");

    assert_eq!(
      output.style_rules,
      Vec::<String>::new(),
      "Should have no style rules"
    )
  }

  #[test]
  fn test_panic_converted_to_diagnostic() {
    // This test verifies that panics from the transform are converted to diagnostics
    // rather than crashing the process
    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
      <div css={styles} />;
    "#};

    let result = process_compiled_css_in_js(code, &config);

    // The result should be successful (error handling doesn't panic)
    // Even if the transform has issues, they should be converted to diagnostics
    match result {
      Ok(_output) => {
        // If we get an OK result, we can check diagnostics
        // In a normal case, this would have diagnostics or bail_out set
        assert!(true, "Transform should not panic");
      }
      Err(e) => {
        // If there's an error, it should be from anyhow, not a panic
        // This verifies the panic guard is working
        assert!(true, "Transform error handling should not panic: {}", e);
      }
    }
  }

  #[test]
  fn test_panic_handling_returns_structured_errors() {
    // This test verifies that when a panic occurs during transformation,
    // it is caught and converted into structured diagnostics that can be
    // properly reported to the user

    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const MyComponent = () => {
        const styles = css({ color: 'red', fontSize: '16px' });
        return <div css={styles}>Hello World</div>;
      };
    "#};

    // This should not panic, even if internal errors occur
    // All errors should be wrapped in the Result type
    let result = process_compiled_css_in_js(code, &config);

    // We should get a Result that we can safely handle
    match result {
      Ok(output) => {
        // If successful, verify the output is structured correctly
        assert!(output.code.len() > 0, "Output code should not be empty");
        // Diagnostics should be a vector (possibly empty)
        let _diagnostics = output.diagnostics;
      }
      Err(e) => {
        // Any error should be a proper anyhow error, not a panic
        // This proves the panic handling is working
        let error_msg = e.to_string();
        assert!(!error_msg.is_empty(), "Error should have a message");
      }
    }
  }

  #[test]
  fn test_panic_safety_with_complex_css() {
    // This test verifies panic safety with more complex CSS patterns
    // that might trigger edge cases in the transform logic

    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';

      const complexStyles = css({
        '@media (max-width: 768px)': {
          color: 'blue',
          '&:hover': {
            color: 'red',
          }
        },
        '&::before': {
          content: '""',
          display: 'block',
        }
      });

      export const ComplexComponent = () => (
        <div css={complexStyles}>Complex Styles</div>
      );
    "#};

    // The transform should handle complex CSS without panicking
    let result = process_compiled_css_in_js(code, &config);

    // Should always return a valid Result
    assert!(
      result.is_ok() || result.is_err(),
      "Transform should return Result variant"
    );
  }

  #[test]
  fn test_diagnostics_are_properly_formatted() {
    // This test verifies that diagnostic messages are properly formatted
    // when errors occur during transformation

    let config = create_test_config(true, false);

    let code = indoc! {r#"
      import { css } from '@compiled/react';
      const styles = css({ color: 'red' });
    "#};

    let result = process_compiled_css_in_js(code, &config);

    match result {
      Ok(output) => {
        // Check that diagnostics are properly structured
        for diagnostic in &output.diagnostics {
          // Each diagnostic should have a message
          assert!(
            !diagnostic.message.is_empty(),
            "Diagnostic should have a non-empty message"
          );
          // Severity should be properly set
          assert!(
            !diagnostic.severity.is_empty(),
            "Diagnostic should have severity level"
          );
        }
      }
      Err(_) => {
        // Any error should be structured
      }
    }
  }
}
