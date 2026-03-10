/// Shared test utilities for request tests.
///
/// Provides a configurable [`MockBundleGraph`] and a [`make_bundle`] helper used across
/// [`package_request`] and [`packaging_request`] tests. Import this module with
/// `use crate::requests::test_utils::*;` inside a `#[cfg(test)]` block.
#[cfg(test)]
pub mod bundle_graph {
  use std::collections::HashMap;
  use std::hash::{Hash, Hasher};
  use std::path::PathBuf;

  use atlaspack_core::bundle_graph::BundleGraph;
  use atlaspack_core::hash::IdentifierHasher;
  use atlaspack_core::types::{
    Asset, Bundle, BundleBehavior, Dependency, Environment, FileType, Target,
  };

  // ---------------------------------------------------------------------------
  // Bundle factory
  // ---------------------------------------------------------------------------

  /// Creates a minimal [`Bundle`] suitable for use in request tests.
  ///
  /// - `id` is used as both the bundle ID and the name stem.
  /// - `hash_reference` should be a `HASH_REF_*` placeholder string.
  /// - `bundle_type` controls which packager path is exercised (use
  ///   `FileType::Other(".test".to_string())` for the no-op test packager).
  pub fn make_bundle(id: &str, hash_reference: &str, bundle_type: FileType) -> Bundle {
    Bundle {
      bundle_behavior: None,
      bundle_type,
      entry_asset_ids: vec![],
      env: Environment::default(),
      hash_reference: hash_reference.to_string(),
      id: id.to_string(),
      is_placeholder: false,
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: Some(format!("{id}.test")),
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      target: Target {
        dist_dir: PathBuf::from("/dist"),
        ..Target::default()
      },
    }
  }

  /// Shorthand for creating a `.test`-type bundle (exercises the no-op test packager path).
  pub fn make_test_bundle(id: &str, hash_reference: &str) -> Bundle {
    make_bundle(id, hash_reference, FileType::Other(".test".to_string()))
  }

  // ---------------------------------------------------------------------------
  // MockBundleGraph
  // ---------------------------------------------------------------------------

  /// A configurable [`BundleGraph`] implementation for use in tests.
  ///
  /// Declare reference edges, inline-child edges, and the bundle universe up front via the
  /// [`MockBundleGraph::builder`] API; the graph then answers trait queries from those maps.
  ///
  /// # Example
  ///
  /// ```rust
  /// let a = make_test_bundle("a", "HASH_REF_aaaa");
  /// let b = make_test_bundle("b", "HASH_REF_bbbb");
  /// let graph = MockBundleGraph::builder()
  ///     .bundles(vec![a.clone(), b.clone()])
  ///     // b's content references a's hash
  ///     .reference("b", "a")
  ///     .build();
  /// ```
  pub struct MockBundleGraph {
    /// All bundles known to the graph (including inline bundles not in the main packaging loop).
    bundles: Vec<Bundle>,
    /// `referencing_id → [referenced_id, ...]` — bundles whose hash appears in another bundle's
    /// content.
    references: HashMap<String, Vec<String>>,
    /// `parent_id → [inline_bundle_id, ...]` — inline bundles contained within a parent bundle.
    inline_children: HashMap<String, Vec<String>>,
  }

  impl MockBundleGraph {
    pub fn builder() -> MockBundleGraphBuilder {
      MockBundleGraphBuilder::default()
    }
  }

  impl BundleGraph for MockBundleGraph {
    fn get_bundles(&self) -> Vec<&Bundle> {
      self.bundles.iter().collect()
    }

    fn get_bundle_assets(&self, _bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
      Ok(vec![])
    }

    fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle> {
      self.bundles.iter().find(|b| b.id == id)
    }

    fn get_public_asset_id(&self, _asset_id: &str) -> Option<&str> {
      None
    }

    fn get_dependencies(&self, _asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
      Ok(vec![])
    }

    fn get_resolved_asset(
      &self,
      _dependency: &Dependency,
      _bundle: &Bundle,
    ) -> anyhow::Result<Option<&Asset>> {
      Ok(None)
    }

    fn is_dependency_skipped(&self, _dependency: &Dependency) -> bool {
      false
    }

    fn get_bundle_hash(&self, bundle: &Bundle) -> u64 {
      let mut state = IdentifierHasher::new();
      bundle.hash(&mut state);
      if let Ok(mut assets) = self.get_bundle_assets(bundle) {
        assets.sort_unstable_by_key(|a| a.id.clone());
        for asset in assets {
          asset.hash(&mut state);
        }
      }
      state.finish()
    }

    fn get_referenced_bundle_ids(&self, bundle: &Bundle) -> Vec<String> {
      self.references.get(&bundle.id).cloned().unwrap_or_default()
    }

    fn get_inline_bundle_ids(&self, bundle: &Bundle) -> Vec<String> {
      self
        .inline_children
        .get(&bundle.id)
        .cloned()
        .unwrap_or_default()
    }
  }

  // ---------------------------------------------------------------------------
  // Builder
  // ---------------------------------------------------------------------------

  #[derive(Default)]
  pub struct MockBundleGraphBuilder {
    bundles: Vec<Bundle>,
    references: HashMap<String, Vec<String>>,
    inline_children: HashMap<String, Vec<String>>,
  }

  impl MockBundleGraphBuilder {
    /// Set the full bundle universe. Include inline bundles here even though they are filtered
    /// from the main packaging loop — `get_bundle_by_id` must be able to find them.
    pub fn bundles(mut self, bundles: Vec<Bundle>) -> Self {
      self.bundles = bundles;
      self
    }

    /// Declare that `referencing` bundle's content embeds `referenced` bundle's hash placeholder.
    /// `referenced` will be placed in an earlier topo level than `referencing`.
    pub fn reference(mut self, referencing: &str, referenced: &str) -> Self {
      self
        .references
        .entry(referencing.to_string())
        .or_default()
        .push(referenced.to_string());
      self
    }

    /// Declare that `parent` bundle contains `inline_child` as an inline bundle.
    pub fn inline_child(mut self, parent: &str, inline_child: &str) -> Self {
      self
        .inline_children
        .entry(parent.to_string())
        .or_default()
        .push(inline_child.to_string());
      self
    }

    pub fn build(self) -> MockBundleGraph {
      MockBundleGraph {
        bundles: self.bundles,
        references: self.references,
        inline_children: self.inline_children,
      }
    }
  }
}
