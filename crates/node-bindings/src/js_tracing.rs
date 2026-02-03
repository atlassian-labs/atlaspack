//! This module exposes the tracing API to JavaScript.

use napi_derive::napi;
use serde_json::Value;
use std::collections::HashMap;
use tracing::field;

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
    let span = tracing::span!(
      tracing::Level::INFO,
      "js_span",
      label = label,
      // Note we can't pass arbitrary fields easily from JS to Rust to create the empty fields required to record later, so instead we have a single
      // optional field that can be populated with arbitrary data later (i.e. a JSON object)
      data = field::Empty
    );

    let id = self.current_id;
    if self.current_id == u32::MAX {
      self.current_id = 0;
    } else {
      self.current_id += 1;
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

  #[napi]
  pub fn record(&mut self, id: SpanId, data: Option<Value>) {
    if let Some(span) = self.current_spans.get(&id) {
      let data_str = data.map(|v| v.to_string()).unwrap_or_default();
      span.record("data", data_str);
    }
  }
}
