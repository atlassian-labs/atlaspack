use std::collections::{HashMap, HashSet};

fn vec_get<T>(v: &[Option<T>], idx: usize) -> Option<&T> {
  v.get(idx).and_then(|o| o.as_ref())
}

fn vec_get_mut<T>(v: &mut [Option<T>], idx: usize) -> Option<&mut T> {
  v.get_mut(idx).and_then(|o| o.as_mut())
}

fn vec_set_len<T>(v: &mut Vec<Option<T>>, new_len: usize) {
  if v.len() < new_len {
    v.resize_with(new_len, || None);
  }
}

use atlaspack_core::{
  asset_graph::AssetGraph,
  types::{MaybeBundleBehavior, Priority},
};
use fixedbitset::FixedBitSet;

/// Configuration knobs for the ideal graph build/analysis.
///
/// This is expected to grow as we implement more of the research doc.
#[derive(Debug, Clone, Default)]
pub struct IdealGraphBuildOptions {
  // Reserved for future tunables.
}

/// Summary stats from building an [`IdealGraph`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdealGraphBuildStats {
  pub assets: usize,
  pub dependencies: usize,
}

/// Compact numeric asset identifier used by the ideal graph algorithm.
///
/// This is intended to be cheap to copy/hash and suitable for indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetKey(pub u32);

/// Bi-directional mapping between the asset graph's string ids and compact [`AssetKey`] values.
///
/// This is the primary boundary where we still pay for String allocation/cloning.
#[derive(Debug, Default, Clone)]
pub struct AssetInterner {
  by_id: HashMap<String, AssetKey>,
  ids: Vec<String>,
}

impl AssetInterner {
  pub fn from_asset_graph(asset_graph: &AssetGraph) -> Self {
    let mut ids: Vec<String> = asset_graph.get_assets().map(|a| a.id.clone()).collect();
    ids.sort();
    ids.dedup();

    let mut by_id = HashMap::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
      let key = AssetKey(u32::try_from(i).expect("too many assets to key"));
      by_id.insert(id.clone(), key);
    }

    Self { by_id, ids }
  }

  pub fn key_for(&self, asset_id: &str) -> Option<AssetKey> {
    self.by_id.get(asset_id).copied()
  }

  pub fn id_for(&self, key: AssetKey) -> &str {
    &self.ids[key.0 as usize]
  }

  pub fn len(&self) -> usize {
    self.ids.len()
  }

  pub fn ids(&self) -> &[String] {
    &self.ids
  }

  pub fn ids_cloned(&self) -> Vec<String> {
    self.ids.clone()
  }

  pub fn is_empty(&self) -> bool {
    self.ids.is_empty()
  }

  /// Allocate a new synthetic asset key for an id that doesn't exist in the asset graph
  /// (e.g. shared bundle root pseudo-assets).
  pub fn insert_synthetic(&mut self, asset_id: String) -> AssetKey {
    let next = AssetKey(u32::try_from(self.ids.len()).expect("too many assets to key"));
    self.by_id.insert(asset_id.clone(), next);
    self.ids.push(asset_id);
    next
  }
}

/// Typed decision event.
///
/// This is intended for debugging/visualization and should not be used for correctness.
///
/// To extend: add new variants. As we build out phases, this becomes the "audit trail"
/// explaining *why* the algorithm made a particular choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionKind {
  // Phase 1 (boundaries)
  BoundaryCreated {
    asset: AssetKey,
    from_asset: AssetKey,
    dependency_id: String,
    priority: Priority,
    type_change: bool,
    isolated: bool,
  },

  // Phase 2 (sync graph)
  SyncEdgeIncluded {
    from_asset: AssetKey,
    to_asset: AssetKey,
  },
  SyncEdgeSkipped {
    from_asset: AssetKey,
    to_asset: AssetKey,
    reason: SyncEdgeSkipReason,
  },

  // Phase 4 (placement)
  BundleRootCreated {
    root_asset: AssetKey,
  },
  AssetAssignedToBundle {
    asset: AssetKey,
    bundle_root: AssetKey,
    reason: AssetAssignmentReason,
  },

  // Phase 6 (availability)
  AvailabilityComputed {
    bundle_root: AssetKey,
    ancestor_assets_len: usize,
  },

  // Phase 8+ (shared bundles)
  SharedBundleCreated {
    shared_bundle_root: AssetKey,
    source_bundle_roots: Vec<AssetKey>,
    asset_count: usize,
  },
  AssetMovedToSharedBundle {
    asset: AssetKey,
    from_bundle_root: AssetKey,
    shared_bundle_root: AssetKey,
  },
  BundleInternalized {
    bundle_root: AssetKey,
  },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetAssignmentReason {
  /// Asset is dominated by exactly one bundle root in the dominator tree.
  DominatorSubtree,
  /// Asset is reachable from multiple roots but only one is eligible after availability filtering.
  SingleEligibleRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEdgeSkipReason {
  NonSyncPriority,
  BoundaryTarget,
  Isolated,
  MissingNode,
}

/// Single decision event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
  /// Monotonically increasing sequence number assigned by the logger.
  pub seq: u64,

  /// Phase name (free-form).
  pub phase: &'static str,

  pub kind: DecisionKind,
}

/// A collection of decisions captured during an algorithm run.
///
/// This is intended for debugging/visualization and should not be used for correctness.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DecisionLog {
  next_seq: u64,
  pub decisions: Vec<Decision>,
}

impl DecisionLog {
  pub fn push(&mut self, phase: &'static str, kind: DecisionKind) {
    let seq = self.next_seq;
    self.next_seq += 1;

    self.decisions.push(Decision { seq, phase, kind });
  }

  pub fn is_empty(&self) -> bool {
    self.decisions.is_empty()
  }
}

/// Bundle graph edge classification (bundle-level).
///
/// This corresponds to the research doc's `EdgeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdealEdgeType {
  Sync,
  Parallel,
  Lazy,
  Conditional,
}

/// Stable bundle identifier used within the ideal graph.
///
/// This is the compact numeric key backing the bundle id string.
///
/// - For named bundles, this is the root asset's [`AssetKey`].
/// - For synthetic bundles (e.g. `@@shared:...`), this is the synthetic key allocated via
///   [`AssetInterner::insert_synthetic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdealBundleId(pub u32);

impl IdealBundleId {
  pub fn from_asset_key(k: AssetKey) -> Self {
    Self(k.0)
  }

  pub fn as_asset_key(self) -> AssetKey {
    AssetKey(self.0)
  }
}

/// A bundle in the ideal graph (zero duplication placement).
///
/// Mirrors the research doc's bundle struct, but starts small and grows as we implement.
#[derive(Debug, Clone)]
pub struct IdealBundle {
  pub id: IdealBundleId,

  /// The asset that created this bundle (entry/boundary). Shared bundles may not have one.
  pub root_asset_id: Option<String>,

  /// Assets assigned to this bundle (no duplication in the ideal phase).
  ///
  /// Indexed by `AssetKey.0 as usize`.
  pub assets: FixedBitSet,

  /// Output bundle type (roughly corresponds to asset file type).
  pub bundle_type: atlaspack_core::types::FileType,

  pub needs_stable_name: bool,
  pub behavior: MaybeBundleBehavior,

  /// Assets known to be available when this bundle loads.
  ///
  /// In the doc, this is computed using the *intersection* rule across parent paths.
  ///
  /// Indexed by `AssetKey.0 as usize`.
  pub ancestor_assets: FixedBitSet,
}

impl IdealBundle {
  /// Convenience: assets available at runtime when this bundle loads.
  ///
  /// In the doc this can include additional sets (e.g. bundle-group/parallel bundles).
  pub fn all_assets_available_from_here(&self) -> FixedBitSet {
    let mut result = self.ancestor_assets.clone();
    result.union_with(&self.assets);
    result
  }
}

/// Output of the ideal graph algorithm.
///
/// This is the primary artifact future optimization/materialization phases will consume.
#[derive(Debug, Default, Clone)]
pub struct IdealGraph {
  /// Bundles with assets assigned (zero duplication).
  ///
  /// Indexed by `IdealBundleId.0 as usize`.
  pub bundles: Vec<Option<IdealBundle>>,

  /// Bundle dependency graph (which bundles load which).
  ///
  /// This is kept for ordered iteration.
  pub bundle_edges: Vec<(IdealBundleId, IdealBundleId, IdealEdgeType)>,

  /// Dedup set for bundle edges (kept in sync with `bundle_edges`).
  pub bundle_edge_set: HashSet<(IdealBundleId, IdealBundleId, IdealEdgeType)>,

  /// Asset -> Bundle mapping.
  ///
  /// Indexed by `AssetKey.0 as usize`.
  pub asset_to_bundle: Vec<Option<IdealBundleId>>,

  /// Mapping between `AssetKey` and the original asset id string.
  pub assets: AssetInterner,

  /// Optional debug information captured during the build.
  pub debug: Option<IdealGraphDebug>,
}

impl IdealGraph {
  fn bundle_slot(id: &IdealBundleId) -> usize {
    id.0 as usize
  }

  fn asset_slot(key: &AssetKey) -> usize {
    key.0 as usize
  }

  pub fn get_bundle(&self, id: &IdealBundleId) -> Option<&IdealBundle> {
    vec_get(&self.bundles, Self::bundle_slot(id))
  }

  pub fn get_bundle_mut(&mut self, id: &IdealBundleId) -> Option<&mut IdealBundle> {
    vec_get_mut(&mut self.bundles, Self::bundle_slot(id))
  }

  pub fn bundle_count(&self) -> usize {
    self.bundles.iter().filter(|b| b.is_some()).count()
  }

  pub fn bundles_iter(&self) -> impl Iterator<Item = (&IdealBundleId, &IdealBundle)> {
    self
      .bundles
      .iter()
      .filter_map(|b| b.as_ref())
      .map(|b| (&b.id, b))
  }

  pub fn asset_bundle(&self, asset: &AssetKey) -> Option<IdealBundleId> {
    vec_get(&self.asset_to_bundle, Self::asset_slot(asset)).copied()
  }

  pub(crate) fn set_asset_bundle(&mut self, asset: AssetKey, bundle: Option<IdealBundleId>) {
    let idx = Self::asset_slot(&asset);
    vec_set_len(&mut self.asset_to_bundle, idx + 1);
    self.asset_to_bundle[idx] = bundle;
  }

  /// Test helper: check whether a bundle contains an asset by its string id.
  pub fn bundle_has_asset(&self, bundle_id: &IdealBundleId, asset_id: &str) -> bool {
    let Some(bundle) = self.get_bundle(bundle_id) else {
      return false;
    };
    let Some(key) = self.assets.key_for(asset_id) else {
      return false;
    };
    bundle.assets.contains(key.0 as usize)
  }

  pub fn create_bundle(&mut self, bundle: IdealBundle) -> anyhow::Result<()> {
    let idx = Self::bundle_slot(&bundle.id);
    vec_set_len(&mut self.bundles, idx + 1);

    anyhow::ensure!(
      self.bundles[idx].is_none(),
      "bundle already exists: {}",
      bundle.id.0
    );
    self.bundles[idx] = Some(bundle);
    Ok(())
  }

  pub fn ensure_bundle(&mut self, bundle: IdealBundle) {
    let idx = Self::bundle_slot(&bundle.id);
    vec_set_len(&mut self.bundles, idx + 1);

    if self.bundles[idx].is_none() {
      self.bundles[idx] = Some(bundle);
    }
  }

  pub fn add_bundle_edge(&mut self, from: IdealBundleId, to: IdealBundleId, ty: IdealEdgeType) {
    // Dedup to keep later phases simpler.
    if !self.bundle_edge_set.insert((from, to, ty)) {
      return;
    }

    self.bundle_edges.push((from, to, ty));
  }

  pub fn remove_bundle_edge(&mut self, from: &IdealBundleId, to: &IdealBundleId) {
    // `remove_bundle_edge` intentionally removes *all* edge types between the two bundles.
    // Since `IdealEdgeType` has a small fixed set of variants, we can remove from the set in O(1).
    for ty in [
      IdealEdgeType::Sync,
      IdealEdgeType::Parallel,
      IdealEdgeType::Lazy,
      IdealEdgeType::Conditional,
    ] {
      self.bundle_edge_set.remove(&(*from, *to, ty));
    }

    self.bundle_edges.retain(|(a, b, _)| a != from || b != to);
  }

  /// Remove a bundle entirely: delete the bundle, remove all edges to/from it,
  /// and remove its assets from `asset_to_bundle`.
  pub fn remove_bundle(&mut self, bundle_id: &IdealBundleId) {
    let idx = Self::bundle_slot(bundle_id);
    let bundle = self.bundles.get_mut(idx).and_then(|s| s.take());

    if let Some(bundle) = bundle {
      for asset_idx in bundle.assets.ones() {
        let key = AssetKey(asset_idx as u32);
        if self.asset_bundle(&key) == Some(*bundle_id) {
          self.set_asset_bundle(key, None);
        }
      }
    }

    self
      .bundle_edges
      .retain(|(a, b, _)| a != bundle_id && b != bundle_id);

    self
      .bundle_edge_set
      .retain(|(a, b, _)| a != bundle_id && b != bundle_id);
  }

  /// Retarget all bundle edges whose `to` is in `targets` to instead point at `new_to`.
  ///
  /// Returns the list of rewritten edges (from, old_to, ty) for decision logging.
  pub fn retarget_incoming_edges(
    &mut self,
    targets: &HashSet<IdealBundleId>,
    new_to: &IdealBundleId,
  ) -> Vec<(IdealBundleId, IdealBundleId, IdealEdgeType)> {
    let mut rewritten = Vec::new();

    for (from, to, ty) in self.bundle_edges.iter_mut() {
      if targets.contains(to) {
        // Keep set consistent while mutating the ordered vec.
        self.bundle_edge_set.remove(&(*from, *to, *ty));

        rewritten.push((*from, *to, *ty));
        *to = *new_to;

        self.bundle_edge_set.insert((*from, *to, *ty));
      }
    }

    // Dedup any edges that became identical, while rebuilding the set.
    self.bundle_edge_set.clear();
    self
      .bundle_edges
      .retain(|(a, b, t)| self.bundle_edge_set.insert((*a, *b, *t)));

    rewritten
  }

  pub fn move_asset_to_bundle(
    &mut self,
    asset: AssetKey,
    to: &IdealBundleId,
  ) -> anyhow::Result<Option<IdealBundleId>> {
    let prev = self.asset_bundle(&asset);

    if let Some(prev_bundle) = prev
      && let Some(b) = self.get_bundle_mut(&prev_bundle)
    {
      b.assets.set(asset.0 as usize, false);
    }

    let to_bundle = self
      .get_bundle_mut(to)
      .ok_or_else(|| anyhow::anyhow!("missing bundle: {}", to.0))?;

    to_bundle.assets.insert(asset.0 as usize);
    self.set_asset_bundle(asset, Some(*to));

    Ok(prev)
  }
}

#[derive(Debug, Clone, Default)]
pub struct IdealGraphDebug {
  pub decisions: DecisionLog,

  /// Mapping for rendering [`AssetKey`] values.
  ///
  /// `asset_ids[key.0 as usize]` yields the original asset id string.
  pub asset_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::{
    asset_graph::AssetGraph,
    types::{Asset, Dependency, Environment, FileType, Target},
  };
  use pretty_assertions::assert_eq;

  /// Sanity test: constructing the minimal asset graph used elsewhere still works.
  #[test]
  fn can_construct_minimal_asset_graph_fixture() {
    let mut asset_graph = AssetGraph::new();

    let target = Target::default();
    let entry_dep = Dependency::entry("entry.js".to_string(), target);
    let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

    let entry_asset = Arc::new(Asset {
      id: "entry_asset".into(),
      file_path: "entry.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let entry_asset_node = asset_graph.add_asset(entry_asset, false);
    asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

    assert_eq!(asset_graph.get_assets().count(), 1);
    assert_eq!(asset_graph.get_dependencies().count(), 1);
  }
}
