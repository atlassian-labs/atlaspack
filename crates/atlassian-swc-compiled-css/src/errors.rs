use serde::{Deserialize, Serialize};
use std::any::Any;
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::sync::Once;
use swc_core::common::Span;

thread_local! {
  /// Thread-local storage for the current span context during transformation.
  /// This is used to attach source location information to panics.
  static CURRENT_TRANSFORM_SPAN: RefCell<Option<Span>> = RefCell::new(None);
}

/// Install a custom panic hook that suppresses panic output.
/// This ensures panics are only reported as structured diagnostics, not printed to stderr.
static PANIC_HOOK_INIT: Once = Once::new();

/// Initialize the panic hook once to suppress default panic output.
/// This should be called early in the transform pipeline.
///
/// The hook is set globally (once per process) and applies to all threads,
/// ensuring panic output is suppressed regardless of which thread the panic occurs in.
pub fn init_panic_suppression() {
  PANIC_HOOK_INIT.call_once(|| {
    let debug_panics = std::env::var("COMPILED_CSS_DEBUG_PANIC").is_ok();
    // Set a global panic hook that suppresses panic output
    // This will apply to all threads in the process
    std::panic::set_hook(Box::new(move |info| {
      // Suppress the default panic hook output - errors are handled as diagnostics.
      // When debugging, optionally log the panic location and a backtrace to stderr.
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

/// Represents a recoverable failure emitted by the compiled CSS transforms.
///
/// This error type is used to collect structured diagnostics from the transform pipeline,
/// including errors that may be derived from panics. Panics caught at the entrypoint level
/// are converted into structured diagnostic output instead of aborting the process.
///
/// # Creating from panics
///
/// When a panic is caught at the entrypoint, use `TransformError::from_panic()` to convert
/// it into a structured error that can be reported to the user:
///
/// ```ignore
/// match panic::catch_unwind(AssertUnwindSafe(|| { /* transform */ })) {
///   Ok(result) => { /* handle result */ }
///   Err(panic_payload) => {
///     return Err(vec![TransformError::from_panic(panic_payload, Some("transform"))])
///   }
/// }
/// ```
///
/// # Capturing source span information
///
/// To attach source location information (span) to panics for better diagnostics,
/// set the current transform span before entering code that might panic:
///
/// ```ignore
/// use crate::errors::{set_transform_span, clear_transform_span, TransformSpanGuard};
///
/// // Option 1: Manual span management
/// set_transform_span(node.span);
/// let result = risky_operation();
/// clear_transform_span();
///
/// // Option 2: RAII guard (preferred - automatic cleanup)
/// let _guard = TransformSpanGuard::new(node.span);
/// let result = risky_operation();  // Guard automatically clears span when dropped
/// ```
///
/// When a panic occurs, `TransformError::from_panic()` will automatically capture
/// the current span context and attach it to the error for precise diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformError {
  pub message: String,
  #[serde(skip)]
  pub span: Option<Span>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hints: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none", rename = "documentationUrl")]
  pub documentation_url: Option<String>,
}

impl TransformError {
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      span: None,
      hints: None,
      documentation_url: None,
    }
  }

  pub fn with_span(message: impl Into<String>, span: Span) -> Self {
    Self {
      message: message.into(),
      span: Some(span),
      hints: None,
      documentation_url: None,
    }
  }

  pub fn with_hints(mut self, hints: Vec<String>) -> Self {
    self.hints = Some(hints);
    self
  }

  pub fn with_documentation_url(mut self, url: impl Into<String>) -> Self {
    self.documentation_url = Some(url.into());
    self
  }

  /// Create a TransformError from a panic payload.
  /// Captures the panic message and the current transform span (if any, and if usable).
  /// If no usable span is available, a note is appended to the message.
  ///
  /// The span can be set using `set_transform_span()` or the `TransformSpanGuard` RAII guard.
  pub fn from_panic(panic_payload: Box<dyn Any + Send>) -> Self {
    let mut message = if let Some(s) = panic_payload.downcast_ref::<String>() {
      s.clone()
    } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
      (*s).to_string()
    } else {
      "Unknown panic - possibly an internal error".to_string()
    };

    // Try to get the current span context from thread-local storage, then clear it so
    // stale spans never leak into later panics.
    let span = match get_transform_span() {
      Some(span) if span_is_usable(&span) => Some(span),
      _ => {
        message.push_str(
          "\n\nInternal error: no source span was captured for this panic. \
           This usually means the transformer failed to attach a span before panicking.",
        );
        None
      }
    };
    clear_transform_span();

    Self {
      message,
      span,
      hints: None,
      documentation_url: None,
    }
  }

  /// Attach a span to a panic error to indicate where it occurred in the source.
  /// This is useful for pinpointing the exact location that triggered the panic.
  pub fn with_panic_span(mut self, span: Span) -> Self {
    self.span = Some(span);
    self
  }
}

fn span_is_usable(span: &Span) -> bool {
  !span.lo().is_dummy() && !span.hi().is_dummy()
}

pub type TransformResult<T> = Result<T, Vec<TransformError>>;

/// Set the current transform span context for panic recovery.
/// This span will be attached to any panics that occur during transformation.
///
/// # Example
/// ```ignore
/// set_transform_span(node.span);
/// // ... code that might panic ...
/// clear_transform_span();
/// ```
pub fn set_transform_span(span: Span) {
  CURRENT_TRANSFORM_SPAN.with(|s| {
    *s.borrow_mut() = Some(span);
  });
}

/// Get the current transform span context.
pub fn get_transform_span() -> Option<Span> {
  CURRENT_TRANSFORM_SPAN.with(|s| *s.borrow())
}

/// Clear the current transform span context.
pub fn clear_transform_span() {
  CURRENT_TRANSFORM_SPAN.with(|s| {
    *s.borrow_mut() = None;
  });
}

/// Guard that automatically clears the transform span when dropped.
/// Useful for RAII pattern to ensure span is cleaned up.
pub struct TransformSpanGuard(Option<Span>);

impl TransformSpanGuard {
  /// Create a new span guard with the given span.
  pub fn new(span: Span) -> Self {
    set_transform_span(span);
    TransformSpanGuard(Some(span))
  }
}

impl Drop for TransformSpanGuard {
  fn drop(&mut self) {
    clear_transform_span();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use swc_core::common::{BytePos, DUMMY_SP};

  #[test]
  fn test_transform_error_new() {
    let error = TransformError::new("Test error message");
    assert_eq!(error.message, "Test error message");
    assert!(error.span.is_none());
  }

  #[test]
  fn test_transform_error_from_panic_string() {
    let panic_payload = Box::new("panic message".to_string()) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);
    assert!(error.message.contains("panic message"));
    assert!(
      error
        .message
        .contains("no source span was captured for this panic")
    );
  }

  #[test]
  fn test_transform_error_from_panic_with_context() {
    let panic_payload = Box::new("critical failure".to_string()) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);
    assert!(error.message.contains("critical failure"));
    assert!(
      error
        .message
        .contains("no source span was captured for this panic")
    );
  }

  #[test]
  fn test_transform_error_from_panic_str() {
    let panic_payload = Box::new("str panic") as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);
    assert!(error.message.contains("str panic"));
  }

  #[test]
  fn test_transform_error_from_panic_unknown() {
    let panic_payload = Box::new(42_u32) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);
    assert!(error.message.contains("Unknown panic"));
  }

  #[test]
  fn test_transform_error_with_panic_span() {
    use swc_core::common::DUMMY_SP;

    let panic_payload = Box::new("test panic with span".to_string()) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload).with_panic_span(DUMMY_SP);

    assert!(error.message.contains("test panic with span"));
    assert!(error.span.is_some());
  }

  #[test]
  fn test_transform_error_span_attachment() {
    use swc_core::common::DUMMY_SP;

    let error = TransformError::new("initial message");
    assert!(error.span.is_none());

    let error_with_span = error.with_panic_span(DUMMY_SP);
    assert!(error_with_span.span.is_some());
    assert_eq!(error_with_span.message, "initial message");
  }

  #[test]
  fn test_transform_span_context() {
    use swc_core::common::DUMMY_SP;

    // Initially no span
    assert!(get_transform_span().is_none());

    // Set span
    set_transform_span(DUMMY_SP);
    assert!(get_transform_span().is_some());

    // Clear span
    clear_transform_span();
    assert!(get_transform_span().is_none());
  }

  #[test]
  fn test_panic_captures_current_span() {
    let span = Span::new(BytePos(1), BytePos(5));
    // Set current span
    set_transform_span(span);

    let panic_payload = Box::new("panic with context".to_string()) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);

    // Panic should have captured the span
    assert_eq!(error.span, Some(span));
    assert!(error.message.contains("panic with context"));

    // Clean up
    clear_transform_span();
  }

  #[test]
  fn test_transform_span_guard() {
    use swc_core::common::DUMMY_SP;

    assert!(get_transform_span().is_none());

    {
      let _guard = TransformSpanGuard::new(DUMMY_SP);
      assert!(get_transform_span().is_some());
    }

    // Guard dropped, span should be cleared
    assert!(get_transform_span().is_none());
  }

  #[test]
  fn test_transform_span_guard_with_panic() {
    let span = Span::new(BytePos(1), BytePos(5));

    {
      let _guard = TransformSpanGuard::new(span);

      let panic_payload = Box::new("guarded panic".to_string()) as Box<dyn Any + Send>;
      let error = TransformError::from_panic(panic_payload);

      // Should have span from guard
      assert_eq!(error.span, Some(span));
    }

    // Span should be cleared after guard drops
    assert!(get_transform_span().is_none());
  }

  #[test]
  fn test_from_panic_clears_span_and_retains_valid_span() {
    let span = Span::new(BytePos(1), BytePos(5));
    set_transform_span(span);

    let panic_payload = Box::new("panic with span".to_string()) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);

    assert!(error.span.is_some());
    assert_eq!(error.span.unwrap(), span);
    assert!(
      get_transform_span().is_none(),
      "span should be cleared after from_panic"
    );
  }

  #[test]
  fn test_from_panic_reports_missing_span_when_dummy() {
    set_transform_span(DUMMY_SP);

    let panic_payload = Box::new("panic without span".to_string()) as Box<dyn Any + Send>;
    let error = TransformError::from_panic(panic_payload);

    assert!(error.span.is_none());
    assert!(
      error.message.contains("no source span was captured"),
      "error message should note missing span"
    );
  }
}
