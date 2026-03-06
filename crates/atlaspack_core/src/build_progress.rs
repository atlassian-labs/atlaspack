use serde::Serialize;

/// Progress events emitted during the build pipeline.
///
/// These events are fired from Rust requests and forwarded to JS
/// reporters (e.g. the CLI reporter) for display.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum BuildProgressEvent {
  /// Asset graph is being built.
  Building {
    /// Number of assets built so far.
    complete_assets: usize,
    /// Number of assets discovered so far (may increase as more are found).
    total_assets: usize,
  },
  /// Bundling has started.
  Bundling,
  /// A bundle has been packaged and optimized.
  PackagingAndOptimizing {
    /// Number of bundles completed so far.
    complete_bundles: usize,
    /// Total number of bundles to package.
    total_bundles: usize,
  },
}

/// Wrapper that adds the static `type: "buildProgress"` field to all events.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildProgressMessage<'a> {
  r#type: &'static str,
  #[serde(flatten)]
  event: &'a BuildProgressEvent,
}

impl BuildProgressEvent {
  /// Serialize to a JSON string for sending to JS reporters.
  pub fn to_json(&self) -> String {
    serde_json::to_string(&BuildProgressMessage {
      r#type: "buildProgress",
      event: self,
    })
    .expect("BuildProgressEvent serialization should not fail")
  }
}
