use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::hash::IdentifierHasher;
use crate::types::{Asset, Bundle, Dependency};

/// Minimal interface for querying a bundle graph.
///
/// This trait is currently used by the JS->Rust bundle graph loading code (`BundleGraphFromJs`).
/// As native bundling is implemented, this will evolve or be replaced.
pub trait BundleGraph {
  fn get_bundles(&self) -> Vec<&Bundle>;

  /// Gets all of the assets "contained" by a Bundle.
  fn get_bundle_assets(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>>;

  /// Helper to get a `Bundle` from an `id`.
  fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle>;

  /// Get the public ID for an asset by its full asset ID.
  ///
  /// Public IDs are shortened, base62-encoded versions of asset IDs used at runtime.
  fn get_public_asset_id(&self, asset_id: &str) -> Option<&str>;

  /// Returns all of the Dependencies for an Asset (the dependencies that the asset requires).
  fn get_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>>;

  /// Resolves a dependency to an asset in a bundle.
  fn get_resolved_asset(
    &self,
    dependency: &Dependency,
    bundle: &Bundle,
  ) -> anyhow::Result<Option<&Asset>>;

  /// Returns whether a dependency was excluded because it had no used symbols.
  fn is_dependency_skipped(&self, dependency: &Dependency) -> bool;

  /// Returns all dependencies that resolve TO the given asset (incoming dependency edges).
  ///
  /// This is the reverse of `get_dependencies`, which returns dependencies FROM an asset.
  /// CSS packaging needs this to determine which bundles import a given asset and to
  /// check conditional import metadata on incoming edges.
  fn get_incoming_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>>;

  /// Returns assets in source order (DFS post-order of the dependency graph).
  ///
  /// If asset A imports asset B, B appears before A. This matches the `bundle.traverse()`
  /// exit-callback order used by the JS packagers and is required for CSS cascade semantics.
  ///
  /// Entry assets appear last.
  ///
  /// **Sibling order**: Children are visited in graph edge-iteration order (usually source
  /// order). Implementations do not sort siblings by source position.
  ///
  /// Order is unspecified for `get_bundle_assets`. Use this method when concatenation
  /// order matters.
  fn get_bundle_assets_in_source_order(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>>;

  /// Returns a stable hash representing the current state of `bundle` and its assets.
  ///
  /// This drives the cache key for [`PackageRequest`].
  #[tracing::instrument(level="trace", skip_all, fields(bundle_id = bundle.id))]
  fn get_bundle_hash(&self, bundle: &Bundle) -> u64 {
    let mut state = IdentifierHasher::new();
    bundle.hash(&mut state);
    if let Ok(mut assets) = self.get_bundle_assets(bundle) {
      assets.sort_unstable_by(|a, b| a.id.cmp(&b.id));
      for asset in assets {
        asset.hash(&mut state);
      }
    }
    state.finish()
  }

  /// Returns the IDs of bundles whose `hash_reference` placeholder is embedded in the content of
  /// `bundle`.
  ///
  /// This drives the topological ordering in [`PackagingRequest`]: a bundle must be packaged
  /// before any bundle that references its hash. Return an empty `Vec` if the graph does not track
  /// reference edges — the caller degrades gracefully to processing all bundles in a single
  /// parallel level.
  fn get_referenced_bundle_ids(&self, bundle: &Bundle) -> Vec<String>;

  /// Returns the set of symbol names used from `asset_id` across the bundle graph.
  ///
  /// Returns `Some(set)` if symbol usage information is available, or `None` if the
  /// implementation does not track used symbols (the default).
  fn get_used_symbols(&self, _asset_id: &str) -> Option<HashSet<String>> {
    None
  }

  /// Returns the IDs of inline bundles (bundle_behavior == Inline | InlineIsolated) that are
  /// directly contained within or referenced by `bundle`.
  ///
  /// Inline bundles are filtered out of the main [`PackagingRequest`] loop and are instead
  /// packaged on-demand by their parent packager. However, they may themselves reference other
  /// (non-inline) bundles via hash_reference placeholders. Those non-inline bundles must be
  /// packaged before the parent bundle runs, so the topo sort must account for this transitive
  /// dependency chain.
  ///
  /// See [`PackagingRequest`] for how this is used to compute effective topo sort dependencies.
  /// Return an empty `Vec` if the graph does not track inline bundle edges.
  fn get_inline_bundle_ids(&self, bundle: &Bundle) -> Vec<String>;
}
