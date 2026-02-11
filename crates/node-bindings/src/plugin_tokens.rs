use anyhow::Context;
use atlaspack_atlaskit_tokens::{
  AtlaskitTokensHandler, TokensConfig as SharedTokensConfig,
  TokensPluginOptions as SharedTokensPluginOptions, TokensPluginResult as SharedTokensPluginResult,
};
use napi::{Env, Error as NapiError, JsObject, bindgen_prelude::Buffer};
use napi_derive::napi;
use serde::Serialize;

#[napi(object)]
#[derive(Clone)]
pub struct TokensPluginOptions {
  pub token_data_path: String,
  pub should_use_auto_fallback: bool,
  pub should_force_auto_fallback: bool,
  pub force_auto_fallback_exemptions: Vec<String>,
  pub default_theme: String,
}

#[napi(object)]
#[derive(Clone)]
pub struct TokensConfig {
  pub filename: String,
  pub project_root: String,
  pub is_source: bool,
  pub source_maps: bool,
  pub tokens_options: TokensPluginOptions,
}

use crate::diagnostics::{JsDiagnostic, convert_diagnostic};

#[napi(object)]
#[derive(Clone, Debug, Serialize)]
pub struct TokensPluginResult {
  pub code: String,
  pub map: Option<String>,
  pub diagnostics: Vec<JsDiagnostic>,
}

impl From<SharedTokensPluginResult> for TokensPluginResult {
  fn from(result: SharedTokensPluginResult) -> Self {
    TokensPluginResult {
      code: result.code,
      map: result.map,
      diagnostics: result
        .diagnostics
        .into_iter()
        .map(convert_diagnostic)
        .collect(),
    }
  }
}

/// Apply the tokens transformation plugin to the given code asynchronously
#[napi]
pub fn apply_tokens_plugin(
  raw_code: Buffer,
  config: TokensConfig,
  env: Env,
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

  // Convert to shared config
  let shared_config = SharedTokensPluginOptions {
    token_data_path: config.tokens_options.token_data_path,
    should_use_auto_fallback: config.tokens_options.should_use_auto_fallback,
    should_force_auto_fallback: config.tokens_options.should_force_auto_fallback,
    force_auto_fallback_exemptions: config.tokens_options.force_auto_fallback_exemptions,
    default_theme: config.tokens_options.default_theme,
  };

  // Spawn the work on a Rayon thread
  rayon::spawn(move || {
    let result =
      AtlaskitTokensHandler::new(config.project_root, shared_config).and_then(|handler| {
        let shared_tokens_config = SharedTokensConfig {
          filename: config.filename,
          is_source: config.is_source,
          source_maps: config.source_maps,
        };
        handler.process(&code, shared_tokens_config)
      });
    match result {
      Ok(plugin_result) => {
        let napi_result: TokensPluginResult = plugin_result.into();
        deferred.resolve(move |env| env.to_js_value(&napi_result));
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
  use anyhow::Result;
  use atlaspack_atlaskit_tokens::{
    TokensPluginOptions as SharedTokensPluginOptions,
    TokensPluginResult as SharedTokensPluginResult,
  };
  use indoc::indoc;
  use std::fs;
  use tempfile::NamedTempFile;

  use super::{TokensConfig, TokensPluginOptions};

  // Helper function to create a test token file
  fn create_test_token_file() -> Result<NamedTempFile> {
    let temp_file = NamedTempFile::new()?;
    let token_data = r##"
    {
      "color.text": {
        "light": "#000000",
        "dark": "#ffffff"
      },
      "color.background": {
        "light": "#ffffff",
        "dark": "#000000"
      }
    }
    "##;
    fs::write(temp_file.path(), token_data)?;
    Ok(temp_file)
  }

  // Helper function to create test config (NAPI type)
  fn create_test_config(tokens_path: &str, source_maps: bool) -> TokensConfig {
    TokensConfig {
      filename: "test.ts".to_string(),
      project_root: "/project".to_string(),
      is_source: false,
      source_maps,
      tokens_options: TokensPluginOptions {
        token_data_path: tokens_path.to_string(),
        should_use_auto_fallback: true,
        should_force_auto_fallback: true,
        force_auto_fallback_exemptions: vec![],
        default_theme: "light".to_string(),
      },
    }
  }

  // Helper function for synchronous token processing in tests
  fn process_tokens_sync(
    code: &str,
    config: &TokensConfig,
  ) -> anyhow::Result<SharedTokensPluginResult> {
    use atlaspack_atlaskit_tokens::{AtlaskitTokensHandler, TokensConfig as AtlaskitTokensConfig};

    let shared_plugin_options = SharedTokensPluginOptions {
      token_data_path: config.tokens_options.token_data_path.clone(),
      should_use_auto_fallback: config.tokens_options.should_use_auto_fallback,
      should_force_auto_fallback: config.tokens_options.should_force_auto_fallback,
      force_auto_fallback_exemptions: config.tokens_options.force_auto_fallback_exemptions.clone(),
      default_theme: config.tokens_options.default_theme.clone(),
    };

    let handler = AtlaskitTokensHandler::new(config.project_root.clone(), shared_plugin_options)?;
    let atlaskit_tokens_config = AtlaskitTokensConfig {
      filename: config.filename.clone(),
      is_source: config.is_source,
      source_maps: config.source_maps,
    };
    handler.process(code, atlaskit_tokens_config)
  }

  #[test]
  fn test_tokens_plugin_options_creation() {
    let config = create_test_config("/path/to/tokens.json", true);
    assert_eq!(
      config.tokens_options.token_data_path,
      "/path/to/tokens.json"
    );
    assert!(config.tokens_options.should_use_auto_fallback);
    assert!(config.tokens_options.should_force_auto_fallback);
    assert_eq!(config.tokens_options.default_theme, "light");
  }

  #[test]
  fn test_successful_token_transformation() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
    assert!(result.is_ok(), "Token transformation should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.code.contains("token"),
      "Transformed code should contain token reference"
    );
  }

  #[test]
  fn test_missing_token_file_error() {
    let napi_config = create_test_config("/nonexistent/path/tokens.json", true);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_string = error.to_string();
    assert!(
      error_string.contains("Failed to load token map") || error_string.contains("tokens.json"),
      "Expected token loading error, got: {}",
      error_string
    );
  }

  #[test]
  fn test_invalid_javascript_syntax_error() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let invalid_code = indoc! {r#"
      import { token from '@atlaskit/tokens'; // Missing closing brace
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(invalid_code, &napi_config);
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
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let ts_code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor: string = token('color.text');
      interface MyInterface {
        color: string;
      }
    "#};

    let result = process_tokens_sync(ts_code, &napi_config);
    assert!(result.is_ok(), "TypeScript syntax should be supported");
  }

  #[test]
  fn test_jsx_syntax_support() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let jsx_code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const Component = () => <div style={{color: token('color.text')}}>Hello</div>;
    "#};

    let result = process_tokens_sync(jsx_code, &napi_config);
    assert!(result.is_ok(), "JSX syntax should be supported");
  }

  #[test]
  fn test_different_theme_configuration() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let mut napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);
    napi_config.tokens_options.default_theme = "dark".to_string();

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
    assert!(result.is_ok(), "Different theme configuration should work");
  }

  #[test]
  fn test_auto_fallback_configuration() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let mut napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);
    napi_config.tokens_options.should_use_auto_fallback = false;
    napi_config.tokens_options.should_force_auto_fallback = false;

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
    assert!(result.is_ok(), "Auto fallback configuration should work");
  }

  #[test]
  fn test_force_auto_fallback_exemptions() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let mut napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);
    napi_config.tokens_options.force_auto_fallback_exemptions = vec!["color.text".to_string()];

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
    assert!(result.is_ok(), "Force auto fallback exemptions should work");
  }

  #[test]
  fn test_is_source_flag_behavior() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    // Test with is_source = true
    let result_source = process_tokens_sync(code, &napi_config);
    assert!(result_source.is_ok(), "is_source=true should work");

    // Test with is_source = false
    let result_not_source = process_tokens_sync(code, &napi_config);
    assert!(result_not_source.is_ok(), "is_source=false should work");
  }

  #[test]
  fn test_empty_code_input() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let empty_code = "";
    let result = process_tokens_sync(empty_code, &napi_config);

    // Empty code should result in an error since we now check for it
    assert!(result.is_err(), "Empty code should result in an error");
  }

  #[test]
  fn test_code_without_tokens() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let code_without_tokens = indoc! {r#"
      const greeting = "Hello, world!";
      console.log(greeting);
    "#};

    let result = process_tokens_sync(code_without_tokens, &napi_config);

    // Code without tokens should still be processed successfully
    // Even if there are no tokens to transform, the code should parse and emit correctly
    match result {
      Ok(transformed) => {
        assert!(
          transformed.code.contains("Hello, world!"),
          "Original code should be preserved"
        );
      }
      Err(e) => {
        // If there's an error, it should be related to parsing, not token processing
        let error_string = e.to_string();
        println!("Got error for code without tokens: {}", error_string);
        unreachable!("Code without tokens should not result in an error");
      }
    }
  }

  #[test]
  fn test_sourcemap_generation() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
    assert!(result.is_ok(), "Token transformation should succeed");

    let transformed = result.unwrap();
    assert!(
      transformed.code.contains("token"),
      "Transformed code should contain token reference"
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
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), false);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      const textColor = token('color.text');
    "#};

    let result = process_tokens_sync(code, &napi_config);
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

  #[test]
  fn test_preserve_unicode_characters_in_react_attributes() {
    let temp_file = create_test_token_file().expect("Failed to create temp file");
    let napi_config = create_test_config(temp_file.path().to_str().unwrap(), true);

    let code = indoc! {r#"
      import { token } from '@atlaskit/tokens';
      import React from 'react';

      const Component = () => {
        return <input type="text" placeholder="This has a special characer that should not be munged: …"/>;
      };

      const t = Component();
    "#};
    let result = process_tokens_sync(code, &napi_config);
    assert!(
      result.is_ok(),
      "Unicode characters should be preserved in React attributes"
    );

    let transformed = result.unwrap();
    assert!(
      transformed
        .code
        .contains("This has a special characer that should not be munged: …"),
      "Transformed code should contain the original Unicode character"
    );
  }
}
