use std::collections::{HashMap, HashSet};

use atlaspack_core::types::{MaybeBundleBehavior, Priority};

/// Configuration knobs for the ideal graph build/analysis.
///
/// This is expected to grow as we implement more of the research doc.
#[derive(Debug, Clone, Default)]
pub struct IdealGraphBuildOptions {
  /// When true, the builder will collect additional debugging metadata.
  pub collect_debug: bool,
}

/// Summary stats from building an [`IdealGraph`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdealGraphBuildStats {
  pub assets: usize,
  pub dependencies: usize,
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
    asset_id: String,
    from_asset_id: String,
    dependency_id: String,
    priority: Priority,
    type_change: bool,
    isolated: bool,
  },

  // Phase 2 (sync graph)
  SyncEdgeIncluded {
    from_asset_id: String,
    to_asset_id: String,
  },
  SyncEdgeSkipped {
    from_asset_id: String,
    to_asset_id: String,
    reason: SyncEdgeSkipReason,
  },

  // Phase 4 (placement)
  BundleRootCreated {
    bundle_id: IdealBundleId,
    root_asset_id: String,
  },
  AssetAssignedToBundle {
    asset_id: String,
    bundle_id: IdealBundleId,
  },

  // Phase 6 (availability)
  AvailabilityComputed {
    bundle_id: IdealBundleId,
    ancestor_assets_len: usize,
  },
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

#[derive(Debug, Clone, Default)]
pub struct IdealGraphDebug {
  pub decisions: DecisionLog,
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
