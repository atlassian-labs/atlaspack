//! This module exposes the tracing API to JavaScript.

use napi_derive::napi;
use std::collections::HashMap;

pub type SpanId = u32;

#[napi]
struct AtlaspackTracer {
  current_spans: HashMap<SpanId, tracing::span::EnteredSpan>,
  current_id: SpanId,
}

#[napi]
impl AtlaspackTracer {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      current_spans: HashMap::new(),
      current_id: 0,
    }
  }

  #[napi]
  pub fn enter(&mut self, label: String) -> SpanId {
    let span = tracing::span!(tracing::Level::INFO, "js_span", label = label);

    let id = self.current_id;
    if self.current_id == u32::MAX {
      self.current_id = 0;
    } else {
      self.current_id = self.current_id + 1;
    }

    self.current_spans.insert(id, span.entered());

    id
  }

  #[napi]
  pub fn exit(&mut self, id: SpanId) {
    if let Some(span) = self.current_spans.remove(&id) {
      drop(span);
    }
  }
}
