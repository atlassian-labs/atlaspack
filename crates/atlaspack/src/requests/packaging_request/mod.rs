// Temporarily required because this request type is not yet wired into the main pipeline
#![allow(dead_code)]

//! Orchestrates packaging of all bundles in the bundle graph.
//!
//! [`PackagingRequest`] is the native equivalent of the JS `WriteBundlesRequest` at the
//! orchestration level. It traverses the bundle graph in topological order — ensuring that every
//! bundle is packaged before any bundle whose content embeds its hash — and fans out to a
//! [`PackageRequest`] for each individual bundle.
//!
//! # Topological ordering
//!
//! Bundle A must be packaged before bundle B if B's content contains A's `hash_reference`
//! placeholder (a `HASH_REF_*` string). The ordering is derived from
//! [`BundleGraph::get_referenced_bundle_ids`], which returns the IDs of bundles that a given
//! bundle references. Bundles at the same topological level have no dependency between them and
//! are dispatched concurrently via [`RunRequestContext::execute_request`].
//!
//! Cycles in the reference graph are a hard error — they indicate a bug in the bundler.

mod topo_sort;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_core::bundle_graph::BundleGraph;
use atlaspack_core::types::{Bundle, BundleBehavior};
use parking_lot::RwLock;

use atlaspack_core::build_progress::BuildProgressEvent;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};
use crate::requests::RequestResult;
use crate::requests::package_request::{PackageRequest, PackageRequestOutput};
use topo_sort::{name_hash_for_filename, topological_levels};

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// The result of packaging all bundles in the bundle graph.
#[derive(Clone, Debug, PartialEq)]
pub struct PackagingRequestOutput {
  /// Maps each bundle ID to the result of packaging that bundle.
  ///
  /// Contains the output file path, content hash, file size in bytes, and packaging duration for
  /// every bundle that was processed.
  pub bundles: HashMap<String, PackageRequestOutput>,
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// Packages all bundles in the bundle graph in dependency order.
///
/// Bundles are processed level-by-level in topological order so that every bundle whose
/// `hash_reference` placeholder appears inside another bundle's content is fully packaged — and
/// its content hash therefore known — before the referencing bundle is packaged.
///
/// Within each topological level, bundles are dispatched concurrently.
///
/// # Errors
///
/// Returns an error if the bundle reference graph contains a cycle, or if any individual
/// [`PackageRequest`] fails.
pub struct PackagingRequest<B: BundleGraph + Send + Sync + 'static> {
  bundle_graph: Arc<RwLock<B>>,
}

impl<B: BundleGraph + Send + Sync + 'static> PackagingRequest<B> {
  /// Creates a new `PackagingRequest` from an owned bundle graph.
  ///
  /// The bundle graph is wrapped in `Arc<RwLock<_>>` internally for thread-safe sharing with
  /// the individual [`PackageRequest`] instances dispatched during packaging.
  pub fn new(bundle_graph: B) -> Self {
    Self {
      bundle_graph: Arc::new(RwLock::new(bundle_graph)),
    }
  }
}

impl<B: BundleGraph + Send + Sync + 'static> std::fmt::Debug for PackagingRequest<B> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PackagingRequest").finish_non_exhaustive()
  }
}

impl<B: BundleGraph + Send + Sync + 'static> Hash for PackagingRequest<B> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    // Mirror JS WriteBundlesRequest: keyed on the hash of all bundles in the bundle graph.
    // If any bundle's content changes, the packaging request is invalidated.
    // We sort bundle hashes for stability — bundle order in the graph is not guaranteed.
    let graph = self.bundle_graph.read();
    let mut bundle_hashes: Vec<u64> = graph
      .get_bundles()
      .into_iter()
      .map(|b| graph.get_bundle_hash(b))
      .collect();
    bundle_hashes.sort_unstable();
    bundle_hashes.hash(state);
  }
}

#[async_trait]
impl<B: BundleGraph + Send + Sync + 'static> Request for PackagingRequest<B> {
  async fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let bundles: Vec<Bundle> = self
      .bundle_graph
      .read()
      .get_bundles()
      .into_iter()
      .filter(|b| {
        // Inline bundles are never written to disk — they are packaged on-demand by their parent
        // packager (e.g. HTML/SVG) via the bundle graph. Including them here would write empty
        // files to disk, which is incorrect. The parent packager is responsible for pulling their
        // content and embedding it directly into its own output.
        if matches!(
          b.bundle_behavior,
          Some(BundleBehavior::Inline) | Some(BundleBehavior::InlineIsolated)
        ) {
          return false;
        }
        // Placeholder bundles are never written to disk either. Their hash_reference is
        // pre-populated in the map below so other bundles can still resolve it.
        if b.is_placeholder {
          return false;
        }
        true
      })
      .cloned()
      .collect();

    let levels = topological_levels(&bundles, &*self.bundle_graph.read())?;

    // Pre-populate hash refs for all bundles using nameHashForFilename(bundle.id) as a stable
    // fallback. This mirrors the JS WriteBundlesRequest's assignComplexNameHashes post-pass.
    //
    // NOTE: In production builds (shouldContentHash: true) this fallback produces incorrect
    // values for bundles whose real filename uses a content hash (last8(content_hash)) rather
    // than last8(bundle.id). The two values will differ, so any JS bundle that URL-imports an
    // SVG/image bundle without a proper References edge in the bundle graph will embed a broken
    // URL. This is a known limitation: the JS bundler does not currently emit References edges
    // for URL-type cross-type dependencies (e.g. JS → SVG). Fixing that requires either
    // correcting the bundler to emit the missing edge, or adopting a two-phase packaging approach
    // (package all bundles first, then resolve hash refs from the extracted hashReferences list).
    // For now we accept the broken reference and focus on simpler build scenarios.
    //
    // Real content hashes (from packaged_hashes, accumulated level-by-level) take priority over
    // these fallbacks for bundles that are properly ordered via References edges.
    let all_bundle_fallbacks: HashMap<String, String> = {
      let graph = self.bundle_graph.read();
      graph
        .get_bundles()
        .into_iter()
        .filter(|b| !b.hash_reference.is_empty())
        .map(|b| (b.hash_reference.clone(), name_hash_for_filename(&b.id)))
        .collect()
    };

    // Real content hashes accumulated level-by-level as bundles are packaged.
    // These overwrite the fallbacks for bundles that are properly ordered via References edges.
    let mut packaged_hashes: HashMap<String, String> = HashMap::new();

    let mut all_outputs: HashMap<String, PackageRequestOutput> =
      HashMap::with_capacity(bundles.len());
    let total_bundles = bundles.len();
    let mut complete_bundles: usize = 0;

    for level in levels {
      // Build the dispatch map: stable name-hash fallbacks for all bundles, then overwrite with
      // real content hashes accumulated from earlier topo levels.
      let mut dispatch_map = all_bundle_fallbacks.clone();
      dispatch_map.extend(packaged_hashes.clone());

      // Dispatch all bundles at this level concurrently.
      let futures: Vec<(String, _)> = level
        .into_iter()
        .map(|bundle| {
          let id = bundle.id.clone();
          let request =
            PackageRequest::new(bundle, Arc::clone(&self.bundle_graph), dispatch_map.clone());
          let future = request_context.execute_request(request);
          (id, future)
        })
        .collect();

      for (bundle_id, future) in futures {
        let (result, _request_id, _cached) = future.await?;
        let output = match result.as_ref() {
          RequestResult::Package(o) => o.clone(),
          other => {
            return Err(anyhow!(
              "Unexpected result type for bundle {bundle_id}: {other}"
            ));
          }
        };
        complete_bundles += 1;
        request_context.report(BuildProgressEvent::PackagingAndOptimizing {
          complete_bundles,
          total_bundles,
        });

        // Record this bundle's real content hash so subsequent levels can resolve it.
        let hash_ref = {
          let graph = self.bundle_graph.read();
          graph
            .get_bundle_by_id(&bundle_id)
            .map(|b| b.hash_reference.clone())
            .unwrap_or_default()
        };
        if !hash_ref.is_empty() {
          packaged_hashes.insert(hash_ref, output.hash.clone());
        }
        all_outputs.insert(bundle_id, output);
      }
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::Packaging(PackagingRequestOutput {
        bundles: all_outputs,
      }),
      invalidations: vec![],
    })
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use pretty_assertions::assert_eq;

  use crate::requests::RequestResult;
  use crate::requests::test_utils::bundle_graph::{MockBundleGraph, make_test_bundle};
  use crate::test_utils::{RequestTrackerTestOptions, request_tracker};

  use super::*;

  fn make_bundle(id: &str, hash_reference: &str) -> Bundle {
    make_test_bundle(id, hash_reference)
  }

  async fn run_packaging_request(graph: MockBundleGraph) -> PackagingRequestOutput {
    let mut rt = request_tracker(RequestTrackerTestOptions::default());
    let request = PackagingRequest::new(graph);
    let result = rt.run_request(request).await.unwrap();
    match result.as_ref() {
      RequestResult::Packaging(output) => output.clone(),
      other => panic!("Expected Packaging result, got {other}"),
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_placeholder_bundle_hash_ref_pre_populated() {
    // Bundle 'p' is a placeholder: it is never packaged but other bundles can embed its
    // HASH_REF_* in their content. The orchestrator must pre-populate the map with a stable
    // name hash (last 8 chars of bundle id) so that packaged bundles can resolve it.
    // Since the test packager arm returns empty content, we verify the run completes without
    // error (i.e. the placeholder's hash ref was pre-populated).
    let mut p = make_bundle("p_placeholder", "HASH_REF_p_placeholder_ref");
    p.is_placeholder = true;
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let graph = MockBundleGraph::builder()
      .bundles(vec![p.clone(), a.clone()])
      .build();
    let output = run_packaging_request(graph).await;
    // 'a' should be packaged; 'p' should not appear in output (it's a placeholder).
    assert!(
      output.bundles.contains_key("a"),
      "non-placeholder bundle should be packaged"
    );
    assert!(
      !output.bundles.contains_key("p_placeholder"),
      "placeholder bundle should not appear in output"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_inline_bundle_hash_ref_pre_populated() {
    // Inline bundles are never written to disk — their hash_reference must still be resolvable
    // via the non-packaged fallback so that parent bundles can embed the reference in filenames.
    let mut inline = make_bundle("inline_id", "HASH_REF_iiiiiiiiiiiiiiii");
    inline.bundle_behavior = Some(BundleBehavior::Inline);
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let graph = MockBundleGraph::builder()
      .bundles(vec![inline.clone(), a.clone()])
      .build();
    let output = run_packaging_request(graph).await;
    assert!(output.bundles.contains_key("a"));
    assert!(
      !output.bundles.contains_key("inline_id"),
      "inline bundle should not appear in output"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_no_bundles() {
    let graph = MockBundleGraph::builder().build();
    let output = run_packaging_request(graph).await;
    assert_eq!(output.bundles, HashMap::new());
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_single_bundle() {
    let bundle = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let graph = MockBundleGraph::builder().bundles(vec![bundle]).build();
    let output = run_packaging_request(graph).await;
    assert!(
      output.bundles.contains_key("a"),
      "bundle 'a' should be in output"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_hash_accumulates_across_levels() {
    // b references a: a is packaged first, its hash becomes available for b.
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let graph = MockBundleGraph::builder()
      .bundles(vec![a.clone(), b.clone()])
      .reference("b", "a")
      .build();
    let output = run_packaging_request(graph).await;
    assert!(output.bundles.contains_key("a"));
    assert!(output.bundles.contains_key("b"));
    assert!(
      !output.bundles["a"].hash.is_empty(),
      "bundle a should have a content hash"
    );
    assert!(
      !output.bundles["b"].hash.is_empty(),
      "bundle b should have a content hash"
    );
  }
}
