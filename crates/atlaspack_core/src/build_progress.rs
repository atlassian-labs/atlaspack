use serde::Serialize;

/// Progress events emitted during the build pipeline.
///
/// These events are fired from Rust requests and forwarded to JS
/// reporters (e.g. the CLI reporter) for display.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum BuildProgressEvent {
  /// Asset graph is being built.
  #[serde(rename_all = "camelCase")]
  Building {
    /// Number of assets built so far.
    complete_assets: usize,
    /// Number of assets discovered so far (may increase as more are found).
    total_assets: usize,
  },
  /// Bundling has started.
  Bundling,
  /// A bundle has been packaged and optimized.
  #[serde(rename_all = "camelCase")]
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_building_event_json() {
    let event = BuildProgressEvent::Building {
      complete_assets: 5,
      total_assets: 10,
    };
    let json = event.to_json();
    assert!(json.contains("\"completeAssets\":5"));
    assert!(json.contains("\"totalAssets\":10"));
    assert!(json.contains("\"phase\":\"building\""));
    assert!(json.contains("\"type\":\"buildProgress\""));
  }

  #[test]
  fn test_bundling_event_json() {
    let event = BuildProgressEvent::Bundling;
    let json = event.to_json();
    assert!(json.contains("\"phase\":\"bundling\""));
    assert!(json.contains("\"type\":\"buildProgress\""));
  }
}
