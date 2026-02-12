use std::collections::{HashMap, HashSet};

use atlaspack_core::types::{MaybeBundleBehavior, Priority};

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
/// For now this is just the root asset id (string). We keep it wrapped so we can
/// change representation later.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdealBundleId(pub String);

/// A bundle in the ideal graph (zero duplication placement).
///
/// Mirrors the research doc's bundle struct, but starts small and grows as we implement.
#[derive(Debug, Clone)]
pub struct IdealBundle {
  pub id: IdealBundleId,

  /// The asset that created this bundle (entry/boundary). Shared bundles may not have one.
  pub root_asset_id: Option<String>,

  /// Assets assigned to this bundle (no duplication in the ideal phase).
  pub assets: HashSet<String>,

  /// Output bundle type (roughly corresponds to asset file type).
  pub bundle_type: atlaspack_core::types::FileType,

  pub needs_stable_name: bool,
  pub behavior: MaybeBundleBehavior,

  /// Assets known to be available when this bundle loads.
  ///
  /// In the doc, this is computed using the *intersection* rule across parent paths.
  pub ancestor_assets: HashSet<String>,
}

impl IdealBundle {
  /// Convenience: assets available at runtime when this bundle loads.
  ///
  /// In the doc this can include additional sets (e.g. bundle-group/parallel bundles).
  pub fn all_assets_available_from_here(&self) -> HashSet<String> {
    self
      .ancestor_assets
      .union(&self.assets)
      .cloned()
      .collect::<HashSet<_>>()
  }
}

/// Output of the ideal graph algorithm.
///
/// This is the primary artifact future optimization/materialization phases will consume.
#[derive(Debug, Default, Clone)]
pub struct IdealGraph {
  /// Bundles with assets assigned (zero duplication).
  pub bundles: HashMap<IdealBundleId, IdealBundle>,

  /// Bundle dependency graph (which bundles load which).
  pub bundle_edges: Vec<(IdealBundleId, IdealBundleId, IdealEdgeType)>,

  /// Asset -> Bundle mapping.
  pub asset_to_bundle: HashMap<String, IdealBundleId>,

  /// Optional debug information captured during the build.
  pub debug: Option<IdealGraphDebug>,
}

impl IdealGraph {
  pub fn create_bundle(&mut self, bundle: IdealBundle) -> anyhow::Result<()> {
    anyhow::ensure!(
      !self.bundles.contains_key(&bundle.id),
      "bundle already exists: {}",
      bundle.id.0
    );
    self.bundles.insert(bundle.id.clone(), bundle);
    Ok(())
  }

  pub fn ensure_bundle(&mut self, bundle: IdealBundle) {
    self.bundles.entry(bundle.id.clone()).or_insert(bundle);
  }

  pub fn add_bundle_edge(&mut self, from: IdealBundleId, to: IdealBundleId, ty: IdealEdgeType) {
    // Dedup to keep later phases simpler.
    if self
      .bundle_edges
      .iter()
      .any(|(a, b, t)| *a == from && *b == to && *t == ty)
    {
      return;
    }
    self.bundle_edges.push((from, to, ty));
  }

  pub fn remove_bundle_edge(&mut self, from: &IdealBundleId, to: &IdealBundleId) {
    self.bundle_edges.retain(|(a, b, _)| a != from || b != to);
  }

  /// Remove a bundle entirely: delete the bundle, remove all edges to/from it,
  /// and remove its assets from `asset_to_bundle`.
  pub fn remove_bundle(&mut self, bundle_id: &IdealBundleId) {
    if let Some(bundle) = self.bundles.remove(bundle_id) {
      for asset_id in &bundle.assets {
        if self.asset_to_bundle.get(asset_id) == Some(bundle_id) {
          self.asset_to_bundle.remove(asset_id);
        }
      }
    }
    self
      .bundle_edges
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
        rewritten.push((from.clone(), to.clone(), *ty));
        *to = new_to.clone();
      }
    }

    // Dedup any edges that became identical.
    let mut seen = HashSet::new();
    self
      .bundle_edges
      .retain(|(a, b, t)| seen.insert((a.clone(), b.clone(), *t)));

    rewritten
  }

  pub fn move_asset_to_bundle(
    &mut self,
    asset_id: &str,
    to: &IdealBundleId,
  ) -> anyhow::Result<Option<IdealBundleId>> {
    let prev = self.asset_to_bundle.get(asset_id).cloned();

    if let Some(prev_bundle) = &prev
      && let Some(b) = self.bundles.get_mut(prev_bundle)
    {
      b.assets.remove(asset_id);
    }

    let to_bundle = self
      .bundles
      .get_mut(to)
      .ok_or_else(|| anyhow::anyhow!("missing bundle: {}", to.0))?;

    to_bundle.assets.insert(asset_id.to_string());
    self
      .asset_to_bundle
      .insert(asset_id.to_string(), to.clone());

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
