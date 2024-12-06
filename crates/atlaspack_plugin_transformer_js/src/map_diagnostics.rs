//! The original JS Transformer uses a similar Diagnostic format
//! this module exports mapper functions to handle converting the
//! original format to one that is understood by Atlaspack V3
use std::path::PathBuf;

use atlaspack_core::types::Code;
use atlaspack_core::types::FileType;
use atlaspack_js_swc_core::utils::Diagnostic as ClassicDiagnostic;

use atlaspack_core::types::CodeFrame;
use atlaspack_core::types::CodeHighlight;
use atlaspack_core::types::Diagnostic;
use atlaspack_core::types::Diagnostics;
use atlaspack_core::types::ErrorKind;
use atlaspack_core::types::Location;

#[derive(Clone)]
pub struct MapDiagnosticOptions {
  pub source_code: Option<Code>,
  pub file_type: Option<FileType>,
  pub file_path: Option<PathBuf>,
}

pub fn map_diagnostics(
  diagnostics: Vec<ClassicDiagnostic>,
  options: MapDiagnosticOptions,
) -> Diagnostics {
  let mut mapped = Diagnostics::default();
  for diagnostic in diagnostics {
    mapped
      .diagnostics
      .push(map_diagnostic(diagnostic, options.clone()))
  }
  mapped
}

pub fn map_diagnostic(
  input: ClassicDiagnostic,
  MapDiagnosticOptions {
    source_code,
    file_type,
    file_path: path,
  }: MapDiagnosticOptions,
) -> Diagnostic {
  let mut diagnostic = Diagnostic {
    kind: ErrorKind::Unknown,
    code_frames: Default::default(),
    hints: input.hints.unwrap_or_default(),
    documentation_url: input.documentation_url.clone(),
    message: input.message,
    origin: Some("@atlaspack/transformer-js".to_string()),
  };

  let source_code = if let Some(source_code) = source_code {
    match source_code.as_str() {
      Ok(code) => Some(code.to_string()),
      Err(_) => None,
    }
  } else {
    None
  };

  if let Some(code_highlights) = input.code_highlights {
    let mut code_frame = CodeFrame {
      code_highlights: vec![],
      code: source_code.clone(),
      language: file_type.map(|v| v.into()),
      file_path: path.clone(),
    };

    for code_highlight in code_highlights {
      code_frame.code_highlights.push(CodeHighlight {
        message: code_highlight.message,
        start: Location {
          line: code_highlight.loc.start_line,
          column: code_highlight.loc.start_col,
        },
        end: Location {
          line: code_highlight.loc.end_line,
          column: code_highlight.loc.end_col,
        },
      })
    }
    diagnostic.code_frames.push(code_frame);
  }

  diagnostic
}
