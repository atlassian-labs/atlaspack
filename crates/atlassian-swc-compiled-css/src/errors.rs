pub use atlaspack_core::types::Diagnostic;
use atlaspack_core::types::{CodeHighlight, DiagnosticBuilder, ErrorKind, Location};
use std::any::Any;
use std::backtrace::Backtrace;
use std::sync::Once;
use swc_core::common::{SourceMap, Span};

/// Install a custom panic hook that suppresses panic output.
static PANIC_HOOK_INIT: Once = Once::new();

/// Initialize the panic hook to suppress default panic output.
/// Panics are converted to diagnostics instead.
pub fn init_panic_suppression() {
  PANIC_HOOK_INIT.call_once(|| {
    let debug_panics = std::env::var("COMPILED_CSS_DEBUG_PANIC").is_ok();
    std::panic::set_hook(Box::new(move |info| {
      if debug_panics {
        eprintln!(
          "[compiled-css] panic: {info}{}",
          info
            .location()
            .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
            .unwrap_or_default()
        );
        eprintln!("{:?}", Backtrace::force_capture());
      }
    }));
  });
}

/// Create a diagnostic with a message and origin set to the given module path.
pub fn create_diagnostic(message: impl Into<String>, origin: &str) -> Diagnostic {
  DiagnosticBuilder::default()
    .message(message.into())
    .kind(ErrorKind::Unknown)
    .origin(Some(origin.to_string()))
    .build()
    .expect("Failed to build diagnostic")
}

/// Convert a panic payload to an Atlaspack Diagnostic.
pub fn diagnostic_from_panic(panic_payload: Box<dyn Any + Send>) -> Diagnostic {
  let message = if let Some(s) = panic_payload.downcast_ref::<String>() {
    s.clone()
  } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
    (*s).to_string()
  } else {
    "Unknown panic".to_string()
  };

  create_diagnostic(message, "compiled-css")
}

/// Convert an SWC span to a CodeHighlight with source locations.
/// Returns None if the span is unusable (dummy span).
#[allow(dead_code)]
fn span_to_code_highlight(span: Span, source_map: &SourceMap) -> Option<CodeHighlight> {
  if span.lo().is_dummy() || span.hi().is_dummy() {
    return None;
  }

  let start = source_map.lookup_char_pos(span.lo);
  let end = source_map.lookup_char_pos(span.hi);

  Some(CodeHighlight {
    message: None,
    start: Location {
      line: start.line,
      column: start.col_display + 1,
    },
    end: Location {
      line: end.line,
      column: end.col_display + 1,
    },
  })
}

pub type TransformResult<T> = Result<T, Vec<Diagnostic>>;
