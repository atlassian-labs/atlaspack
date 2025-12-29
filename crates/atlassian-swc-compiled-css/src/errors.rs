use swc_core::common::Span;

/// Represents a recoverable failure emitted by the compiled CSS transforms.
#[derive(Debug, Clone)]
pub struct TransformError {
  pub message: String,
  pub span: Option<Span>,
}

impl TransformError {
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      span: None,
    }
  }

  pub fn with_span(message: impl Into<String>, span: Span) -> Self {
    Self {
      message: message.into(),
      span: Some(span),
    }
  }
}

pub type TransformResult<T> = Result<T, Vec<TransformError>>;
