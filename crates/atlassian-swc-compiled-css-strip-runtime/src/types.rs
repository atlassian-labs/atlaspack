use serde::{Deserialize, Serialize};
use swc_core::ecma::ast::Program;

/// Rust representation of the Babel strip-runtime plugin options.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PluginOptions {
  pub style_sheet_path: Option<String>,
  pub compiled_require_exclude: Option<bool>,
  pub extract_styles_to_directory: Option<ExtractStylesToDirectory>,
  pub sort_at_rules: Option<bool>,
  pub sort_shorthand: Option<bool>,
}

impl Default for PluginOptions {
  fn default() -> Self {
    Self {
      style_sheet_path: None,
      compiled_require_exclude: None,
      extract_styles_to_directory: None,
      sort_at_rules: None,
      sort_shorthand: None,
    }
  }
}

/// Configuration passed from the JS binding mirroring the Babel plugin entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TransformConfig {
  pub filename: Option<String>,
  pub cwd: Option<String>,
  pub root: Option<String>,
  #[serde(rename = "sourceFileName")]
  pub source_file_name: Option<String>,
  pub options: PluginOptions,
}

impl Default for TransformConfig {
  fn default() -> Self {
    Self {
      filename: None,
      cwd: None,
      root: None,
      source_file_name: None,
      options: PluginOptions::default(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractStylesToDirectory {
  pub source: String,
  pub dest: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransformMetadata {
  pub style_rules: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransformOutput {
  pub program: Program,
  pub metadata: TransformMetadata,
}

impl TransformOutput {
  pub fn empty(program: Program) -> Self {
    Self {
      program,
      metadata: TransformMetadata::default(),
    }
  }
}
