pub use atlaspack_core::types::Diagnostic;
use atlaspack_core::types::{CodeHighlight, DiagnosticBuilder, ErrorKind, Location};
use std::any::Any;
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::sync::Once;
use swc_core::common::{SourceMap, Span};

thread_local! {
  /// Thread-local storage for the most recent panic backtrace.
  /// This is set by the panic hook and retrieved when converting panics to diagnostics.
  static LAST_PANIC_BACKTRACE: RefCell<Option<String>> = RefCell::new(None);
}

/// Install a custom panic hook that suppresses panic output.
static PANIC_HOOK_INIT: Once = Once::new();

/// Initialize the panic hook to suppress default panic output.
/// Panics are converted to diagnostics instead.
pub fn init_panic_suppression() {
  PANIC_HOOK_INIT.call_once(|| {
    let debug_panics = std::env::var("COMPILED_CSS_DEBUG_PANIC").is_ok();
    std::panic::set_hook(Box::new(move |info| {
      // Capture backtrace at the panic site (before catch_unwind discards it)
      let backtrace = Backtrace::force_capture();
      let backtrace_str = format!("{:?}", backtrace);

      // Store it in thread-local storage for diagnostic_from_panic to retrieve
      LAST_PANIC_BACKTRACE.with(|bt| {
        *bt.borrow_mut() = Some(backtrace_str.clone());
      });

      if debug_panics {
        eprintln!(
          "[compiled-css] panic: {info}{}",
          info
            .location()
            .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
            .unwrap_or_default()
        );
        eprintln!("{}", backtrace_str);
      }
    }));
  });
}

/// Create a diagnostic with a message and origin set to the given module path.
/// Optionally accepts a span and source_map to include source code location.
pub fn create_diagnostic(
  message: impl Into<String>,
  origin: &str,
  span: Option<Span>,
  source_map: Option<&SourceMap>,
) -> Diagnostic {
  use atlaspack_core::types::CodeFrame;

  let code_frames = match (span, source_map) {
    (Some(span), Some(sm)) => span_to_code_highlight(span, sm)
      .map(|highlight| {
        vec![CodeFrame {
          code_highlights: vec![highlight],
          code: None,
          language: None,
          file_path: None,
        }]
      })
      .unwrap_or_default(),
    _ => vec![],
  };

  DiagnosticBuilder::default()
    .message(message.into())
    .kind(ErrorKind::ParseError)
    .origin(Some(origin.to_string()))
    .code_frames(code_frames)
    .build()
    .expect("Failed to build diagnostic")
}

/// Convert a panic payload to an Atlaspack Diagnostic.
/// Includes backtrace information as a hint for debugging.
pub fn diagnostic_from_panic(panic_payload: Box<dyn Any + Send>) -> Diagnostic {
  let message = if let Some(s) = panic_payload.downcast_ref::<String>() {
    s.clone()
  } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
    (*s).to_string()
  } else {
    "Unknown panic".to_string()
  };

  // Retrieve the backtrace captured at the panic site (from the panic hook)
  let backtrace_str = LAST_PANIC_BACKTRACE.with(|bt| {
    bt.borrow_mut().take().unwrap_or_else(|| {
      // Fallback: capture backtrace here if panic hook didn't run
      let bt = Backtrace::force_capture();
      format!("{:?}", bt)
    })
  });

  DiagnosticBuilder::default()
    .message(message)
    .kind(ErrorKind::ParseError)
    .origin(Some("compiled-css".to_string()))
    .hints(vec![format!("Stack trace:\n{}", backtrace_str)])
    .build()
    .expect("Failed to build diagnostic")
}

/// Convert an SWC span to a CodeHighlight with source locations.
/// Returns None if the span is unusable (dummy span).
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
