//! Shared utilities for converting Rust diagnostics to NAPI-compatible JavaScript types.
//!
//! This module provides generic NAPI structs and conversion functions that can be used
//! by any plugin that needs to return diagnostics from Rust/SWC to TypeScript.

use atlaspack_js_swc_core::utils::{CodeHighlight, Diagnostic, DiagnosticSeverity, SourceLocation};
use napi_derive::napi;
use serde::Serialize;

/// NAPI-compatible source location for JavaScript
#[napi(object)]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsSourceLocation {
  pub start_line: u32,
  pub start_col: u32,
  pub end_line: u32,
  pub end_col: u32,
}

/// NAPI-compatible code highlight for JavaScript
#[napi(object)]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsCodeHighlight {
  pub message: Option<String>,
  pub loc: JsSourceLocation,
}

/// NAPI-compatible diagnostic for JavaScript
#[napi(object)]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsDiagnostic {
  pub message: String,
  pub code_highlights: Option<Vec<JsCodeHighlight>>,
  pub hints: Option<Vec<String>>,
  pub show_environment: bool,
  pub severity: String,
  pub documentation_url: Option<String>,
}

/// Convert a Rust SourceLocation to a NAPI-compatible JsSourceLocation
pub fn convert_source_location(loc: SourceLocation) -> JsSourceLocation {
  JsSourceLocation {
    start_line: loc.start_line as u32,
    start_col: loc.start_col as u32,
    end_line: loc.end_line as u32,
    end_col: loc.end_col as u32,
  }
}

/// Convert a Rust CodeHighlight to a NAPI-compatible JsCodeHighlight
pub fn convert_code_highlight(highlight: CodeHighlight) -> JsCodeHighlight {
  JsCodeHighlight {
    message: highlight.message,
    loc: convert_source_location(highlight.loc),
  }
}

/// Convert a Rust Diagnostic to a NAPI-compatible JsDiagnostic
pub fn convert_diagnostic(diagnostic: Diagnostic) -> JsDiagnostic {
  JsDiagnostic {
    message: diagnostic.message,
    code_highlights: diagnostic
      .code_highlights
      .map(|highlights| highlights.into_iter().map(convert_code_highlight).collect()),
    hints: diagnostic.hints,
    show_environment: diagnostic.show_environment,
    severity: severity_to_string(diagnostic.severity),
    documentation_url: diagnostic.documentation_url,
  }
}

/// Convert a vector of Rust Diagnostics to a vector of NAPI-compatible JsDiagnostics
pub fn convert_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<JsDiagnostic> {
  diagnostics.into_iter().map(convert_diagnostic).collect()
}

/// Convert severity enum to string
pub fn severity_to_string(severity: DiagnosticSeverity) -> String {
  match severity {
    DiagnosticSeverity::Error => "Error".to_string(),
    DiagnosticSeverity::Warning => "Warning".to_string(),
    DiagnosticSeverity::SourceError => "SourceError".to_string(),
  }
}
