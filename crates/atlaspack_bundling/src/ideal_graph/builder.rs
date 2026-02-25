use fixedbitset::FixedBitSet;
use roaring::RoaringBitmap;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SyncNode {
  VirtualRoot,
  Asset(super::types::AssetKey),
}

/// Compact reachability representation using bitsets.
///
/// `reach_bits[node_index]` contains a bitset where bit `i` is set if `roots[i]` can reach that node.
struct Reachability {
  reach_bits: Vec<FixedBitSet>,
  roots: Vec<AssetKey>,
  root_to_bit: HashMap<AssetKey, usize>,
}

use anyhow::Context;
use atlaspack_core::{
  asset_graph::{AssetGraph, DependencyState, NodeId},
  types::{BundleBehavior, EnvironmentContext, Priority},
};
use petgraph::{
  Direction,
  algo::kosaraju_scc,
  graph::{DiGraph, NodeIndex},
  stable_graph::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use tracing::{debug, instrument};

use super::types::{
  AssetKey, BundleRootEdgeType, IdealBundle, IdealBundleId, IdealEdgeType, IdealGraph,
  IdealGraphBuildOptions, IdealGraphBuildStats,
};

/// When true, the bundler skips dependencies marked as Deferred or Excluded by symbol propagation.
/// Controlled by the `ATLASPACK_SKIP_UNUSED_DEPENDENCIES` env var (default: true).
/// Both JS and Rust bundlers read this same env var to stay in sync for parity testing.
/// Set to false (e.g. `ATLASPACK_SKIP_UNUSED_DEPENDENCIES=false`) when testing Rust bundler
/// parity, since the Rust side does not yet have symbol propagation.
fn skip_unused_dependencies() -> bool {
  std::env::var("ATLASPACK_SKIP_UNUSED_DEPENDENCIES")
    .map(|v| v != "false")
    .unwrap_or(true)
}

/// Intermediate state for building an [`IdealGraph`].
///
/// This mirrors the phase-based design in `bundler-rust-rewrite-research.md`.
#[derive(Debug, Default)]
pub struct IdealGraphBuilder {
  pub options: IdealGraphBuildOptions,

  // Decision log is always-on: cheap, allocation-free payloads.
  decisions: super::types::DecisionLog,

  // Phase outputs / caches
  assets: super::types::AssetInterner,
  bundle_boundaries: HashSet<super::types::AssetKey>,

  sync_graph: StableDiGraph<SyncNode, ()>,
  virtual_root: Option<NodeIndex>,
  asset_to_sync_node: HashMap<super::types::AssetKey, NodeIndex>,
  /// Sync graph nodes for non-entry assets with `bundle_behavior.is_some()`.
  /// Maps to the asset key so we can look up the node's own root bit.
  bundle_behavior_sync_nodes: HashMap<NodeIndex, AssetKey>,

  // Graph of bundle root assets with Lazy/Parallel edges.
  // Includes a virtual root connected to all entry bundle roots.
  bundle_root_graph: DiGraph<AssetKey, BundleRootEdgeType>,
  bundle_root_graph_nodes: HashMap<AssetKey, NodeIndex>,
  bundle_root_graph_virtual_root: Option<NodeIndex>,

  // Bundle-root graph node ordering matching the JS reference implementation:
  // iterative DFS post-order starting from `bundle_root_graph_virtual_root`.
  //
  // This intentionally differs from SCC-condensation ordering: cycles are handled by
  // skipping already-visited nodes (like JS topoSort).
  bundle_root_graph_topo_order: Vec<AssetKey>,

  // Entry roots.
  entry_roots: HashSet<super::types::AssetKey>,

  // Bundle roots are either entry assets or boundary assets.
  bundle_roots: HashSet<super::types::AssetKey>,

  // Asset key -> file type, populated during asset interning.
  asset_file_types: HashMap<super::types::AssetKey, atlaspack_core::types::FileType>,

  // Only assets reachable from the current target entry roots.
  // Matches JS bundler behavior where non-target entry dependency subtrees are skipped.
  reachable_assets: HashSet<super::types::AssetKey>,

  // Bundle roots that should be treated like entries (assets duplicated into them).
  // Includes entries, non-splittable, isolated, and stable-name bundle roots.
  entry_like_roots: HashSet<super::types::AssetKey>,

  // Type-change sibling bundles: keyed by (parent_bundle_root, file_type_str).
  // When placing a CSS asset into a JS bundle, redirect into a sibling bundle.
  type_change_siblings: HashMap<(AssetKey, String), IdealBundleId>,
}

impl IdealGraphBuilder {
  fn decision(&mut self, phase: &'static str, kind: super::types::DecisionKind) {
    self.decisions.push(phase, kind);
  }

  pub fn new(options: IdealGraphBuildOptions) -> Self {
    Self {
      options,
      decisions: super::types::DecisionLog::default(),
      type_change_siblings: HashMap::new(),
      bundle_behavior_sync_nodes: HashMap::new(),
      ..Self::default()
    }
  }

  /// Full pipeline entrypoint.
  #[instrument(level = "debug", skip_all)]
  pub fn build(
    mut self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<(IdealGraph, IdealGraphBuildStats)> {
    self.assets = super::types::AssetInterner::from_asset_graph(asset_graph);
    for asset in asset_graph.get_assets() {
      if let Some(key) = self.assets.key_for(&asset.id) {
        self.asset_file_types.insert(key, asset.file_type.clone());
      }
    }
    debug!(
      interned_assets = self.assets.len(),
      "ideal graph: interned asset ids"
    );
    let entry_assets = self.extract_entry_assets(asset_graph)?;
    self.entry_roots = entry_assets.iter().copied().collect();

    // Compute which assets are reachable from the current target entries.
    // This must run before any phase that iterates over assets.
    self.compute_reachable_assets(asset_graph, &entry_assets)?;

    // Phase 0: gather core stats.
    let stats = IdealGraphBuildStats {
      assets: asset_graph.get_assets().count(),
      dependencies: asset_graph.get_dependencies().count(),
    };

    debug!(
      assets = stats.assets,
      dependencies = stats.dependencies,
      entries = entry_assets.len(),
      "ideal graph: input stats"
    );

    // Phase 1: identify bundle boundaries.
    self.identify_bundle_boundaries(asset_graph)?;

    // Phase 2: build sync-only graph.
    self.build_sync_graph(asset_graph, &entry_assets)?;

    // Phase 3: create bundle root shells (no non-root asset placement yet).
    let mut ideal = self.create_bundle_roots(asset_graph, &entry_assets)?;

    // Phase 4: bundle dependency graph (edge types) + bundle group assignment.
    self.build_bundle_edges(asset_graph, &mut ideal)?;

    // Phase 5: derive reachability via topological bitset propagation.
    let mut reachability = self.compute_reachability_topological()?;

    // Phase 5b: build bundle-root graph (lazy/parallel edges) for availability computation.
    self.build_bundle_root_graph(asset_graph)?;

    // Phase 6: availability propagation (ancestor_assets).
    //
    // Note: Phase 6 placement needs `ancestor_assets` to avoid placing assets that are already
    // available from an ancestor bundle (matches JS Insert Or Share filtering).
    self.compute_availability(&reachability, &mut ideal)?;

    // Phase 7: internalize async bundles whose root is already available from all parents.
    // Must run before placement so internalized roots are removed from reachability
    // (matching JS where internalization runs before Insert Or Share).
    self.internalize_async_bundles(asset_graph, &mut reachability, &mut ideal)?;

    // Phase 8: place single-root assets into their dominating bundle.
    self.place_single_root_assets(&reachability, &mut ideal)?;

    // Phase 9: extract shared bundles for multi-root assets.
    self.create_shared_bundles(&reachability, &mut ideal)?;

    // Always attach debug info. Decision payloads are compact and avoid String cloning.
    ideal.debug = Some(super::types::IdealGraphDebug {
      decisions: std::mem::take(&mut self.decisions),
      asset_ids: ideal.assets.ids_cloned(),
    });

    Ok((ideal, stats))
  }

  // ----------------------------
  // Phase 0: Entry extraction
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn extract_entry_assets(
    &self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<Vec<super::types::AssetKey>> {
    let mut entries: Vec<super::types::AssetKey> = Vec::new();

    // We detect entry dependencies via `dep.is_entry` and then follow outgoing edges
    // from the dependency node to the asset node.
    //
    // Note: `add_entry_dependency` does not implicitly connect to the graph root.
    // Therefore, we do *not* rely on root connectivity here.
    for dep in asset_graph.get_dependencies() {
      if !dep.is_entry {
        continue;
      }

      let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dep.id).copied() else {
        continue;
      };

      for maybe_entry_asset in asset_graph.get_outgoing_neighbors(&dep_node_id) {
        if let Some(asset) = asset_graph.get_asset(&maybe_entry_asset)
          && let Some(key) = self.assets.key_for(&asset.id)
        {
          entries.push(key);
        }
      }
    }

    anyhow::ensure!(!entries.is_empty(), "asset graph had no entry assets");
    debug!(entries = entries.len(), "ideal graph: extracted entries");
    Ok(entries)
  }

  // ----------------------------
  // Reachable assets
  // ----------------------------

  /// Compute the set of assets reachable from the current target entry roots.
  ///
  /// Mirrors the JS bundler behavior where subtrees under non-target entry dependencies
  /// are skipped.
  #[instrument(level = "debug", skip_all, fields(entries = entry_assets.len()))]
  fn compute_reachable_assets(
    &mut self,
    asset_graph: &AssetGraph,
    entry_assets: &[AssetKey],
  ) -> anyhow::Result<()> {
    self.reachable_assets.clear();

    let mut queue: VecDeque<NodeId> = VecDeque::new();

    for &entry_key in entry_assets {
      self.reachable_assets.insert(entry_key);

      let entry_asset_id = self.assets.id_for(entry_key);
      if let Some(node_id) = asset_graph
        .get_node_id_by_content_key(entry_asset_id)
        .copied()
      {
        queue.push_back(node_id);
      }
    }

    while let Some(asset_node_id) = queue.pop_front() {
      for dep_node_id in asset_graph.get_outgoing_neighbors(&asset_node_id) {
        let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
          continue;
        };

        // Match JS `skipUnusedDependencies: true`.
        let state = asset_graph.get_dependency_state(&dep_node_id);
        if skip_unused_dependencies()
          && matches!(state, DependencyState::Deferred | DependencyState::Excluded)
        {
          continue;
        }

        for target_asset_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
          let Some(target_asset) = asset_graph.get_asset(&target_asset_node_id) else {
            continue;
          };

          let Some(target_key) = self.assets.key_for(&target_asset.id) else {
            continue;
          };

          // Skip subtrees under entry dependencies that are not part of the current target entries.
          if dep.is_entry && !self.entry_roots.contains(&target_key) {
            continue;
          }

          if self.reachable_assets.insert(target_key) {
            queue.push_back(target_asset_node_id);
          }
        }
      }
    }

    debug!(
      reachable = self.reachable_assets.len(),
      "ideal graph: reachable assets computed"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 1: Bundle boundaries
  // ----------------------------

  /// Identify assets that should become bundle roots (bundle boundaries).
  ///
  /// Mirrors the research doc rules:
  /// - async/lazy imports
  /// - conditional imports
  /// - type changes
  /// - isolated bundle behavior
  /// - parallel imports
  #[instrument(level = "debug", skip_all)]
  fn identify_bundle_boundaries(&mut self, asset_graph: &AssetGraph) -> anyhow::Result<()> {
    self.bundle_boundaries.clear();

    for asset_node_id in self.asset_node_ids(asset_graph) {
      let Some(from_asset) = asset_graph.get_asset(&asset_node_id) else {
        continue;
      };

      let Some(from_key) = self.assets.key_for(&from_asset.id) else {
        continue;
      };

      if !self.reachable_assets.contains(&from_key) {
        continue;
      }

      for dep_node_id in asset_graph.get_outgoing_neighbors(&asset_node_id) {
        let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
          continue;
        };

        // Match JS `skipUnusedDependencies: true`.
        let state = asset_graph.get_dependency_state(&dep_node_id);
        if skip_unused_dependencies()
          && matches!(state, DependencyState::Deferred | DependencyState::Excluded)
        {
          continue;
        }

        for target_asset_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
          let Some(target_asset) = asset_graph.get_asset(&target_asset_node_id) else {
            continue;
          };

          let inline = dep.bundle_behavior == Some(BundleBehavior::Inline)
            || target_asset.bundle_behavior == Some(BundleBehavior::Inline);
          let dep_is_boundary = dep.priority != Priority::Sync || inline;
          let type_change = from_asset.file_type != target_asset.file_type;
          let isolated = dep.bundle_behavior == Some(BundleBehavior::Isolated)
            || dep.bundle_behavior == Some(BundleBehavior::InlineIsolated)
            || target_asset.bundle_behavior == Some(BundleBehavior::Isolated)
            || target_asset.bundle_behavior == Some(BundleBehavior::InlineIsolated);

          // Sync type-change deps (e.g. `import './style.css'` from JS) should NOT create
          // bundle boundaries. They are regular sync-reachable assets that get separated
          // into type-change sibling bundles during Phase 6 placement.
          //
          // Non-sync type-change (e.g. lazy CSS, which doesn't exist in practice) and
          // isolated type-change (SVG via `new URL()`) ARE boundaries.
          //
          // This matches the JS bundler's `idealGraph.ts` where CSS assets from sync
          // imports are NOT bundle roots — they flow through the normal placement/sharing
          // phases and get separated by type in `addAssetToBundleRoot`.
          let is_sync_type_change = type_change && !dep_is_boundary && !isolated && !inline;
          if (dep_is_boundary || type_change || isolated || inline) && !is_sync_type_change {
            let Some(target_key) = self.assets.key_for(&target_asset.id) else {
              continue;
            };

            if !self.reachable_assets.contains(&target_key) {
              continue;
            }

            let inserted = self.bundle_boundaries.insert(target_key);
            if inserted {
              self.decision(
                "boundaries",
                super::types::DecisionKind::BoundaryCreated {
                  asset: target_key,
                  from_asset: from_key,
                  dependency_id: dep.id.clone(),
                  priority: dep.priority,
                  type_change,
                  isolated,
                },
              );
            }
          }
        }
      }
    }

    debug!(
      boundaries = self.bundle_boundaries.len(),
      "ideal graph: identified boundaries"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 2: Build sync graph
  // ----------------------------

  #[instrument(level = "debug", skip_all, fields(entries = entry_assets.len()))]
  fn build_sync_graph(
    &mut self,
    asset_graph: &AssetGraph,
    entry_assets: &[AssetKey],
  ) -> anyhow::Result<()> {
    self.sync_graph = StableDiGraph::new();
    self.asset_to_sync_node.clear();
    self.bundle_behavior_sync_nodes.clear();

    let virtual_root = self.sync_graph.add_node(SyncNode::VirtualRoot);
    self.virtual_root = Some(virtual_root);

    // Add reachable assets as nodes.
    for asset_id in self.assets.ids().iter() {
      let Some(key) = self.assets.key_for(asset_id) else {
        continue;
      };
      if !self.reachable_assets.contains(&key) {
        continue;
      }
      let idx = self.sync_graph.add_node(SyncNode::Asset(key));
      self.asset_to_sync_node.insert(key, idx);
    }

    // Connect virtual root to entry assets.
    for &entry in entry_assets {
      let Some(&entry_idx) = self.asset_to_sync_node.get(&entry) else {
        continue;
      };
      self.sync_graph.add_edge(virtual_root, entry_idx, ());
    }

    // Also connect virtual root to bundle boundary roots.
    //
    // This ensures dominators/placement work within each boundary-rooted sync component.
    for &boundary in self.bundle_boundaries.iter() {
      let Some(&boundary_idx) = self.asset_to_sync_node.get(&boundary) else {
        continue;
      };
      self.sync_graph.add_edge(virtual_root, boundary_idx, ());
    }

    // Add edges for sync dependencies only, stopping at bundle boundaries and isolated bundles.
    for asset_node_id in self.asset_node_ids(asset_graph) {
      let Some(from_asset) = asset_graph.get_asset(&asset_node_id) else {
        continue;
      };
      let Some(from_key) = self.assets.key_for(&from_asset.id) else {
        continue;
      };

      if !self.reachable_assets.contains(&from_key) {
        continue;
      }

      // Note: JS stops reachability traversal at bundleBehavior assets (skipChildren),
      // but still creates sync edges FROM them. The filtering is done in reachability
      // propagation (compute_reachability_topological), not here in the sync graph.

      let Some(&from_idx) = self.asset_to_sync_node.get(&from_key) else {
        continue;
      };

      if from_asset.bundle_behavior.is_some() && !self.entry_roots.contains(&from_key) {
        self.bundle_behavior_sync_nodes.insert(from_idx, from_key);
      }

      for dep_node_id in asset_graph.get_outgoing_neighbors(&asset_node_id) {
        let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
          continue;
        };

        if dep.priority != Priority::Sync {
          continue;
        }

        if dep.bundle_behavior == Some(BundleBehavior::Isolated)
          || dep.bundle_behavior == Some(BundleBehavior::InlineIsolated)
          || dep.bundle_behavior == Some(BundleBehavior::Inline)
        {
          continue;
        }

        // Match JS `skipUnusedDependencies: true`.
        let state = asset_graph.get_dependency_state(&dep_node_id);
        if skip_unused_dependencies()
          && matches!(state, DependencyState::Deferred | DependencyState::Excluded)
        {
          continue;
        }

        for target_asset_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
          let Some(target_asset) = asset_graph.get_asset(&target_asset_node_id) else {
            continue;
          };

          let Some(target_key) = self.assets.key_for(&target_asset.id) else {
            continue;
          };

          if !self.reachable_assets.contains(&target_key) {
            continue;
          }

          // Note: sync type-change edges (e.g. JS → CSS) are NOT skipped here.
          // CSS assets from sync imports are regular sync-reachable assets (not
          // boundaries), so they naturally participate in the sync graph. They
          // will be separated into type-change sibling bundles during Phase 6.
          let Some(&to_idx) = self.asset_to_sync_node.get(&target_key) else {
            continue;
          };

          self.sync_graph.add_edge(from_idx, to_idx, ());
        }
      }
    }

    debug!(
      sync_nodes = self.sync_graph.node_count(),
      sync_edges = self.sync_graph.edge_count(),
      "ideal graph: built sync graph"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 3: Placement
  // ----------------------------

  /// Phase 4: Create bundle root shells only.
  ///
  /// Non-root assets are left unplaced; they will be assigned in Phase 8 using
  /// dominator subtree information combined with availability/reachability data.
  #[instrument(level = "debug", skip_all, fields(entries = entry_assets.len()))]
  fn create_bundle_roots(
    &mut self,
    asset_graph: &AssetGraph,
    entry_assets: &[AssetKey],
  ) -> anyhow::Result<IdealGraph> {
    self.bundle_roots.clear();
    self.bundle_roots.extend(entry_assets.iter().cloned());
    self
      .bundle_roots
      .extend(self.bundle_boundaries.iter().cloned());

    self.entry_like_roots = self.entry_roots.clone();

    let mut ideal = IdealGraph {
      bundles: Vec::new(),
      bundle_edges: Vec::new(),
      bundle_edge_set: HashSet::new(),
      internalized_bundles: HashMap::new(),
      asset_to_bundle: Vec::new(),
      assets: self.assets.clone(),
      debug: None,
    };

    // Hot-path lookups.
    let asset_by_id: HashMap<&str, &atlaspack_core::types::Asset> = asset_graph
      .get_assets()
      .map(|a| (a.id.as_str(), a))
      .collect();

    // Precompute: which assets are targeted by at least one dependency with `needs_stable_name`.
    // This avoids scanning all dependencies for every bundle root.
    let mut assets_needing_stable_name: HashSet<String> = HashSet::new();
    for dep in asset_graph.get_dependencies() {
      if dep.is_entry || !dep.needs_stable_name {
        continue;
      }

      let Some(dep_node) = asset_graph.get_node_id_by_content_key(&dep.id) else {
        continue;
      };

      for neighbor in asset_graph.get_outgoing_neighbors(dep_node) {
        if let Some(asset) = asset_graph.get_asset(&neighbor) {
          assets_needing_stable_name.insert(asset.id.clone());
        }
      }
    }

    // Avoid cloning the whole set into the loop body while still allowing mutation of
    // `entry_like_roots`.
    let roots: Vec<AssetKey> = self.bundle_roots.iter().copied().collect();

    for root_key in roots {
      let root_asset_id = self.assets.id_for(root_key).to_string();

      let asset = asset_by_id
        .get(root_asset_id.as_str())
        .copied()
        .context("bundle root asset missing from graph")?;

      // Determine if this root should be treated like an entry (assets duplicated into it).
      let is_entry = self.entry_roots.contains(&root_key);
      let is_isolated = asset.bundle_behavior == Some(BundleBehavior::Isolated)
        || asset.bundle_behavior == Some(BundleBehavior::InlineIsolated);

      let dep_needs_stable_name = assets_needing_stable_name.contains(&root_asset_id);

      let needs_stable_name = is_entry || dep_needs_stable_name;

      if !asset.is_bundle_splittable || is_isolated || dep_needs_stable_name {
        self.entry_like_roots.insert(root_key);
      }

      let bundle_id = IdealBundleId::from_asset_key(root_key);

      ideal.create_bundle(IdealBundle {
        id: bundle_id,
        root_asset_id: Some(root_asset_id.clone()),
        bundle_group_root: None,
        assets: {
          let mut bs = FixedBitSet::with_capacity(self.assets.len());
          bs.insert(root_key.0 as usize);
          bs
        },
        bundle_type: asset.file_type.clone(),
        needs_stable_name,
        behavior: asset.bundle_behavior,
        ancestor_assets: RoaringBitmap::new(),
      })?;

      ideal.move_asset_to_bundle(root_key, &bundle_id)?;

      self.decision(
        "placement",
        super::types::DecisionKind::BundleRootCreated {
          root_asset: root_key,
        },
      );
    }

    debug!(
      bundles = ideal.bundle_count(),
      "ideal graph: created bundle root shells"
    );
    Ok(ideal)
  }

  // ----------------------------
  // Phase 5: Bundle edge graph
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn build_bundle_edges(
    &self,
    asset_graph: &AssetGraph,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    ideal.bundle_edges.clear();
    ideal.bundle_edge_set.clear();

    // Precompute a mapping from every asset to its containing bundle.
    // Bundle roots map to their own bundle; non-root assets are resolved by
    // walking incoming sync edges (BFS) to the nearest ancestor bundle root.
    // This is done once instead of per-edge to avoid repeated allocations.
    let asset_to_containing_bundle: HashMap<AssetKey, IdealBundleId> = {
      let mut map = HashMap::with_capacity(self.assets.len());

      // Seed with all placed assets (bundle roots).
      for key in self.bundle_roots.iter() {
        if let Some(bundle_id) = ideal.asset_bundle(key) {
          map.insert(*key, bundle_id);
        }
      }

      // For unplaced assets, BFS up incoming sync edges to find nearest root.
      let mut queue: VecDeque<NodeIndex> = VecDeque::new();
      let mut visited: HashSet<NodeIndex> = HashSet::new();

      for (&asset_key, &sync_node) in &self.asset_to_sync_node {
        if map.contains_key(&asset_key) {
          continue;
        }

        queue.clear();
        visited.clear();
        queue.push_back(sync_node);

        while let Some(node) = queue.pop_front() {
          if !visited.insert(node) {
            continue;
          }
          if let Some(SyncNode::Asset(key)) = self.sync_graph.node_weight(node)
            && let Some(&bundle_id) = map.get(key)
          {
            map.insert(asset_key, bundle_id);
            break;
          }
          for pred in self
            .sync_graph
            .neighbors_directed(node, Direction::Incoming)
          {
            if let Some(SyncNode::Asset(_)) = self.sync_graph.node_weight(pred) {
              queue.push_back(pred);
            }
          }
        }
      }

      map
    };

    // Seed entry bundles with their own bundle group.
    for &root in &self.entry_roots {
      let bundle_id = IdealBundleId::from_asset_key(root);
      if let Some(bundle) = ideal.get_bundle_mut(&bundle_id) {
        bundle.bundle_group_root = Some(root);
      }
    }

    for asset_node_id in self.asset_node_ids(asset_graph) {
      let Some(from_asset) = asset_graph.get_asset(&asset_node_id) else {
        continue;
      };

      let Some(from_key) = self.assets.key_for(&from_asset.id) else {
        continue;
      };

      let Some(&from_bundle) = asset_to_containing_bundle.get(&from_key) else {
        continue;
      };

      for dep_node_id in asset_graph.get_outgoing_neighbors(&asset_node_id) {
        let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
          continue;
        };

        let edge_type = match dep.priority {
          Priority::Sync => IdealEdgeType::Sync,
          Priority::Parallel => IdealEdgeType::Parallel,
          Priority::Lazy => IdealEdgeType::Lazy,
          Priority::Conditional => IdealEdgeType::Conditional,
        };

        for target_asset_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
          let Some(target_asset) = asset_graph.get_asset(&target_asset_node_id) else {
            continue;
          };

          // Only add bundle-level edges when the target asset is a bundle root.
          let Some(target_key) = self.assets.key_for(&target_asset.id) else {
            continue;
          };

          if !self.bundle_roots.contains(&target_key) {
            continue;
          }

          let to_bundle = IdealBundleId::from_asset_key(target_key);
          if from_bundle == to_bundle {
            continue;
          }

          ideal.add_bundle_edge(from_bundle, to_bundle, edge_type);

          // Assign bundle group root. Own-group edges (lazy/conditional/isolated)
          // always win over inherited-group edges (sync/parallel), regardless of
          // encounter order. This matches JS semantics where DFS order naturally
          // processes lazy edges first, but Rust iterates edges in arbitrary order.
          let target_behavior = target_asset.bundle_behavior;
          let is_own_group = matches!(edge_type, IdealEdgeType::Lazy | IdealEdgeType::Conditional)
            || matches!(
              target_behavior,
              Some(BundleBehavior::Isolated) | Some(BundleBehavior::InlineIsolated)
            );

          let current_group = ideal
            .get_bundle(&to_bundle)
            .and_then(|b| b.bundle_group_root);
          let current_is_own = current_group == Some(target_key);

          // Only assign if:
          // - No group assigned yet, OR
          // - Current group is inherited and this edge wants own group
          if current_group.is_none() || (is_own_group && !current_is_own) {
            let group_root = if is_own_group {
              Some(target_key)
            } else if target_behavior == Some(BundleBehavior::Inline) {
              None
            } else {
              // Sync/parallel targets inherit the parent bundle's group.
              ideal
                .get_bundle(&from_bundle)
                .and_then(|b| b.bundle_group_root)
            };
            if let Some(bundle) = ideal.get_bundle_mut(&to_bundle) {
              bundle.bundle_group_root = group_root;
            }
          }
        }
      }
    }

    // Fallback: any remaining bundles with a root asset that weren't reached
    // by bundle edges start their own group (except Inline bundles).
    for slot in ideal.bundles.iter_mut() {
      let Some(bundle) = slot.as_mut() else {
        continue;
      };
      if bundle.bundle_group_root.is_some() {
        continue;
      }
      if bundle.behavior == Some(BundleBehavior::Inline) {
        continue;
      }
      if bundle.root_asset_id.is_some() {
        bundle.bundle_group_root = Some(bundle.id.as_asset_key());
      }
    }

    debug!(
      edges = ideal.bundle_edges.len(),
      "ideal graph: built bundle edges"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 5b: Bundle-root graph (lazy/parallel edges)
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn build_bundle_root_graph(&mut self, asset_graph: &AssetGraph) -> anyhow::Result<()> {
    self.bundle_root_graph = DiGraph::new();
    self.bundle_root_graph_nodes.clear();

    // NOTE: The node weight type is `AssetKey`, so we represent the virtual root as a sentinel
    // that should never be used for indexing into `assets`-indexed vectors/bitsets.
    let virtual_root_key = AssetKey(u32::MAX);
    let virtual_root = self.bundle_root_graph.add_node(virtual_root_key);
    self.bundle_root_graph_virtual_root = Some(virtual_root);

    // Add a node for each bundle root asset.
    let mut roots: Vec<AssetKey> = self.bundle_roots.iter().copied().collect();
    roots.sort();
    for root in &roots {
      let idx = self.bundle_root_graph.add_node(*root);
      self.bundle_root_graph_nodes.insert(*root, idx);
    }

    // Virtual root -> entry roots.
    let mut entry_roots: Vec<AssetKey> = self.entry_roots.iter().copied().collect();
    entry_roots.sort();
    for entry in entry_roots {
      if let Some(&to_idx) = self.bundle_root_graph_nodes.get(&entry) {
        // Virtual root edges have no semantic edge type in JS (untyped edges).
        // We store them as `Lazy` for now since consumers will treat the virtual root specially.
        self
          .bundle_root_graph
          .add_edge(virtual_root, to_idx, BundleRootEdgeType::Lazy);
      }
    }

    // Precompute `AssetKey -> asset graph NodeId` lookup.
    let mut asset_node_by_key: HashMap<AssetKey, NodeId> = HashMap::new();
    for asset_node_id in self.asset_node_ids(asset_graph) {
      if let Some(asset) = asset_graph.get_asset(&asset_node_id)
        && let Some(key) = self.assets.key_for(&asset.id)
      {
        asset_node_by_key.insert(key, asset_node_id);
      }
    }

    // Precompute env context and bundle behavior for all assets.
    // We need this to:
    // - avoid traversing past isolated assets during per-root sync traversal
    // - match JS edge filtering (`bundle.bundleBehavior == null` and env context checks)
    let mut asset_info: HashMap<AssetKey, (EnvironmentContext, Option<BundleBehavior>)> =
      HashMap::new();
    for asset in asset_graph.get_assets() {
      let Some(key) = self.assets.key_for(&asset.id) else {
        continue;
      };
      asset_info.insert(key, (asset.env.context, asset.bundle_behavior));
    }

    let env_is_isolated = |ctx: EnvironmentContext| -> bool {
      matches!(
        ctx,
        EnvironmentContext::ServiceWorker
          | EnvironmentContext::WebWorker
          | EnvironmentContext::Worklet
          | EnvironmentContext::Tesseract
      )
    };

    // Track (from, to, edge type) to avoid inserting duplicate edges.
    //
    // `petgraph::DiGraph::add_edge` always creates a new edge even if an identical one already exists.
    // The JS bundler deduplicates these edges, and without it we can end up with huge duplicate edge
    // counts which then slow down availability computation.
    let mut seen_edges: HashSet<(NodeIndex, NodeIndex, BundleRootEdgeType)> = HashSet::new();

    // For each root, traverse the asset graph using a stack-based DFS.
    //
    // This mirrors the JS reference implementation:
    // - full asset graph traversal (not sync_graph BFS)
    // - `skipUnusedDependencies: true` (skip Deferred/Excluded dependency subtrees)
    // - dependency nodes are visited interleaved with asset nodes
    for root in roots {
      let Some(&from_node) = self.bundle_root_graph_nodes.get(&root) else {
        continue;
      };

      let Some((root_ctx, _root_behavior)) = asset_info.get(&root).copied() else {
        continue;
      };

      let Some(&root_asset_node_id) = asset_node_by_key.get(&root) else {
        continue;
      };

      // Stack-based DFS (LIFO) to match JS traverse.
      let mut stack: Vec<NodeId> = Vec::new();
      let mut visited: HashSet<NodeId> = HashSet::new();
      stack.push(root_asset_node_id);

      while let Some(node_id) = stack.pop() {
        if !visited.insert(node_id) {
          continue;
        }

        // Asset node handling.
        if let Some(asset) = asset_graph.get_asset(&node_id) {
          let Some(asset_key) = self.assets.key_for(&asset.id) else {
            continue;
          };

          // Skip traversing children past isolated assets (matches JS `asset.bundleBehavior != null`).
          // But don't skip the ROOT of this DFS.
          if asset_key != root && asset.bundle_behavior.is_some() {
            continue;
          }

          // Traverse children (dependency nodes) in insertion order.
          // We push in reverse so the first neighbor is processed first.
          let children: Vec<NodeId> = asset_graph.get_outgoing_neighbors(&node_id);
          for child in children.into_iter().rev() {
            if !visited.contains(&child) {
              stack.push(child);
            }
          }

          continue;
        }

        // Dependency node handling.
        if let Some(dep) = asset_graph.get_dependency(&node_id) {
          // JS `skipUnusedDependencies: true`
          let state = asset_graph.get_dependency_state(&node_id);
          if skip_unused_dependencies()
            && matches!(state, DependencyState::Deferred | DependencyState::Excluded)
          {
            continue;
          }

          if dep.priority == Priority::Sync {
            // Continue DFS into sync target assets.
            //
            // IMPORTANT: Sync dependencies with bundle behavior (Isolated/InlineIsolated/Inline)
            // are bundle boundaries and must not be traversed for availability propagation.
            // Otherwise, the boundary root would be treated as "reachable" from its parent,
            // which can lead to internalization/shared-extraction bugs.
            if matches!(
              dep.bundle_behavior,
              Some(BundleBehavior::Isolated)
                | Some(BundleBehavior::InlineIsolated)
                | Some(BundleBehavior::Inline)
            ) {
              continue;
            }

            let children: Vec<NodeId> = asset_graph.get_outgoing_neighbors(&node_id);
            for child in children.into_iter().rev() {
              if !visited.contains(&child) {
                stack.push(child);
              }
            }
            continue;
          }

          // Non-sync deps: add edge for Lazy/Parallel when they point to bundle roots,
          // then skip children (do not traverse into that subtree).
          let edge_ty = match dep.priority {
            Priority::Lazy => Some(BundleRootEdgeType::Lazy),
            Priority::Parallel => Some(BundleRootEdgeType::Parallel),
            _ => None,
          };

          let Some(edge_ty) = edge_ty else {
            continue;
          };

          for target_asset_node_id in asset_graph.get_outgoing_neighbors(&node_id) {
            let Some(target_asset) = asset_graph.get_asset(&target_asset_node_id) else {
              continue;
            };

            let Some(target_key) = self.assets.key_for(&target_asset.id) else {
              continue;
            };

            if !self.bundle_roots.contains(&target_key) {
              continue;
            }

            // Filter: match JS `bundle.bundleBehavior == null` and `!bundle.env.isIsolated()`.
            // Keep existing bundleBehavior checks on dep and target.
            if dep.bundle_behavior.is_some() || target_asset.bundle_behavior.is_some() {
              continue;
            }

            if env_is_isolated(target_asset.env.context) {
              continue;
            }

            // Env context mismatch check.
            if target_asset.env.context != root_ctx {
              continue;
            }

            let Some(&to_node) = self.bundle_root_graph_nodes.get(&target_key) else {
              continue;
            };

            // Deduplicate edges (see `seen_edges` above).
            if seen_edges.insert((from_node, to_node, edge_ty)) {
              self.bundle_root_graph.add_edge(from_node, to_node, edge_ty);
            }
          }

          continue;
        }

        // Root/Entry nodes (or any other node types) are ignored.
      }
    }

    // Precompute JS-matching topo order for availability computation.
    self.bundle_root_graph_topo_order = self.compute_bundle_root_graph_topo_order();

    debug!(
      bundle_root_nodes = self.bundle_root_graph.node_count(),
      bundle_root_edges = self.bundle_root_graph.edge_count(),
      topo_order_len = self.bundle_root_graph_topo_order.len(),
      "ideal graph: built bundle root graph"
    );

    Ok(())
  }

  // ----------------------------
  // Phase 6: Availability propagation
  // ----------------------------

  /// Compute bundle-root graph order via iterative DFS post-order.
  ///
  /// Requirements (must match JS):
  /// - start from `bundle_root_graph_virtual_root`
  /// - stack-based DFS (no recursion)
  /// - process children in petgraph insertion order
  /// - push children in reverse order onto the stack so the first child is processed first
  /// - on exit/backtrack, append node's `AssetKey`
  /// - skip already visited nodes (cycle back-edges)
  /// - reverse post-order result to get topo order
  fn compute_bundle_root_graph_topo_order(&self) -> Vec<AssetKey> {
    let Some(virtual_root) = self.bundle_root_graph_virtual_root else {
      return Vec::new();
    };

    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut post_order: Vec<AssetKey> = Vec::new();

    // (node, exiting)
    let mut stack: Vec<(NodeIndex, bool)> = Vec::new();
    stack.push((virtual_root, false));

    while let Some((node, exiting)) = stack.pop() {
      if exiting {
        if node != virtual_root
          && let Some(&key) = self.bundle_root_graph.node_weight(node)
        {
          post_order.push(key);
        }
        continue;
      }

      if !visited.insert(node) {
        continue;
      }

      stack.push((node, true));

      // Children in insertion order, then pushed reversed for LIFO.
      let children: Vec<NodeIndex> = self
        .bundle_root_graph
        .edges_directed(node, Direction::Outgoing)
        .map(|e| e.target())
        .collect();

      for child in children.into_iter().rev() {
        if !visited.contains(&child) {
          stack.push((child, false));
        }
      }
    }

    post_order.reverse();
    post_order
  }

  /// Computes `ancestor_assets` for each bundle root using the JS bundler semantics.
  ///
  /// This propagates availability down the `bundle_root_graph` (lazy/parallel edges) in
  /// topological order, intersecting availability through all parent paths.
  ///
  #[instrument(level = "debug", skip_all)]
  fn compute_availability(
    &mut self,
    reachability: &Reachability,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Reset: start with empty availability for all bundles.
    for slot in ideal.bundles.iter_mut() {
      if let Some(bundle) = slot.as_mut() {
        bundle.ancestor_assets.clear();
      }
    }

    let Some(virtual_root) = self.bundle_root_graph_virtual_root else {
      return Ok(());
    };

    // NodeIndex -> computed ancestor assets for that bundle root.
    // Only populated for nodes we actually process.
    let mut ancestor_by_node: HashMap<NodeIndex, RoaringBitmap> = HashMap::new();

    // Initialize entries to empty ancestor set (matches JS `ancestorAssets[entry] = empty`).
    for entry in self.entry_roots.iter() {
      if let Some(&idx) = self.bundle_root_graph_nodes.get(entry) {
        ancestor_by_node.insert(idx, RoaringBitmap::new());
      }
    }

    // Precompute reachable assets per root bit so we don't scan all assets for every root.
    //
    // `reachable_assets_per_root[bit]` contains all assets sync-reachable from `reachability.roots[bit]`.
    // JS semantics: assets with `bundleBehavior != null` are NOT included in `reachableAssets`
    // (JS returns before `reachable.add()` at line 732-735 in idealGraph.ts). They act as
    // traversal boundaries but are never counted as "reachable" from upstream roots.
    // However, a bundleBehavior asset IS reachable from its own root (it is a bundle root).
    let reachable_assets_per_root: Vec<RoaringBitmap> = {
      let num_roots = reachability.roots.len();
      let mut per_root: Vec<RoaringBitmap> = (0..num_roots).map(|_| RoaringBitmap::new()).collect();

      for (&asset_key, &sync_idx) in &self.asset_to_sync_node {
        // Check if this asset has bundleBehavior set.
        let has_bundle_behavior = self.bundle_behavior_sync_nodes.contains_key(&sync_idx);

        let bs = &reachability.reach_bits[sync_idx.index()];
        for bit in bs.ones() {
          // Skip bundleBehavior assets for OTHER roots (matches JS exclusion).
          // A bundleBehavior asset's own root bit is allowed (it's a bundle root).
          if has_bundle_behavior {
            if let Some(&root_key) = reachability.roots.get(bit) {
              if root_key != asset_key {
                continue;
              }
            }
          }

          if let Some(slot) = per_root.get_mut(bit) {
            slot.insert(asset_key.0);
          }
        }
      }

      per_root
    };

    // Determine processing order.
    //
    // Must match JS `topoSort` exactly: DFS post-order from the virtual root, using
    // insertion-order edges (no sorting) and skipping already-visited nodes.
    let order: Vec<NodeIndex> = self
      .bundle_root_graph_topo_order
      .iter()
      .filter_map(|k| self.bundle_root_graph_nodes.get(k).copied())
      .collect();

    // Precompute bundle group membership.
    //
    // JS idealGraph semantics: a bundle group's bundles are always loaded together.
    // When computing availability for a given bundle root, the JS bundler unions in
    // sync-reachable assets from *all* bundles in the same bundle group.
    //
    // In Rust, the `bundle_root_graph` encodes this via Lazy/Parallel edges:
    // - A node with an incoming Parallel edge is in the same bundle group as its parent.
    // - Otherwise, the node starts its own bundle group.
    let mut bundle_group_of: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    let mut bundle_group_members: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();

    for &node in &order {
      if node == virtual_root {
        continue;
      }

      // If there are multiple incoming Parallel edges, choose a deterministic parent.
      let mut parallel_parents: Vec<NodeIndex> = self
        .bundle_root_graph
        .edges_directed(node, Direction::Incoming)
        .filter(|e| *e.weight() == BundleRootEdgeType::Parallel)
        .map(|e| e.source())
        .collect();

      parallel_parents.sort_by_key(|parent| {
        self
          .bundle_root_graph
          .node_weight(*parent)
          .copied()
          .unwrap_or(AssetKey(0))
      });

      let group_root = if let Some(parent) = parallel_parents.first().copied() {
        bundle_group_of.get(&parent).copied().unwrap_or(parent)
      } else {
        node
      };

      bundle_group_of.insert(node, group_root);
      bundle_group_members
        .entry(group_root)
        .or_default()
        .push(node);
    }

    // De-dup members per group for stability.
    for (group_root, members) in bundle_group_members.iter_mut() {
      members.sort_by_key(|n| {
        self
          .bundle_root_graph
          .node_weight(*n)
          .copied()
          .unwrap_or(AssetKey(0))
      });
      members.dedup();

      // Ensure the group root itself is always included.
      if !members.contains(group_root) {
        members.push(*group_root);
        members.sort_by_key(|n| {
          self
            .bundle_root_graph
            .node_weight(*n)
            .copied()
            .unwrap_or(AssetKey(0))
        });
      }
    }

    for node in order {
      if node == virtual_root {
        continue;
      }

      let Some(&root_key) = self.bundle_root_graph.node_weight(node) else {
        continue;
      };

      let bundle_id = IdealBundleId::from_asset_key(root_key);
      let Some(bundle) = ideal.get_bundle(&bundle_id) else {
        continue;
      };

      // Compute `available` for this node.
      let mut available = if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
        || bundle.behavior == Some(BundleBehavior::Inline)
      {
        RoaringBitmap::new()
      } else {
        ancestor_by_node
          .get(&node)
          .cloned()
          .unwrap_or_else(RoaringBitmap::new)
      };

      // Bundle-group co-load: union in reachable assets from ALL bundles in this node's bundle group.
      //
      // Matches JS `idealGraph.ts`:
      //   for bundleIdInGroup of [bundleGroupId, ...bundleGraph.getNodeIdsConnectedFrom(bundleGroupId)]
      //     reachableAssets[bundleIdInGroup.root] are all available.
      if bundle.behavior != Some(BundleBehavior::Isolated)
        && bundle.behavior != Some(BundleBehavior::InlineIsolated)
        && bundle.behavior != Some(BundleBehavior::Inline)
      {
        let group_root = bundle_group_of.get(&node).copied().unwrap_or(node);
        if let Some(members) = bundle_group_members.get(&group_root) {
          for &member in members {
            let Some(&member_key) = self.bundle_root_graph.node_weight(member) else {
              continue;
            };

            // Skip bundles with an explicit behavior (matches JS `bundleBehavior != null`).
            let member_bundle_id = IdealBundleId::from_asset_key(member_key);
            if let Some(member_bundle) = ideal.get_bundle(&member_bundle_id)
              && member_bundle.behavior.is_some()
            {
              continue;
            }

            // Union in member's full sync-reachable set.
            if let Some(&member_bit) = reachability.root_to_bit.get(&member_key)
              && let Some(reachable) = reachable_assets_per_root.get(member_bit)
            {
              available |= reachable;
            }

            // The member root itself is always available.
            available.insert(member_key.0);
          }
        }
      }

      // Persist computed ancestor assets back onto the IdealBundle.
      if let Some(b) = ideal.get_bundle_mut(&bundle_id) {
        b.ancestor_assets = ancestor_by_node
          .get(&node)
          .cloned()
          .unwrap_or_else(RoaringBitmap::new);

        self.decision(
          "availability",
          super::types::DecisionKind::AvailabilityComputed {
            bundle_root: bundle_id.as_asset_key(),
            ancestor_assets_len: b.ancestor_assets.len() as usize,
          },
        );
      }

      // Propagate to children.
      // Maintain per-parent parallel sibling availability: later parallel siblings see earlier siblings.
      let mut parallel_availability = RoaringBitmap::new();

      // Children must be processed in insertion order (matches JS `getNodeIdsConnectedFrom`).
      for e in self
        .bundle_root_graph
        .edges_directed(node, Direction::Outgoing)
      {
        let child = e.target();
        let edge_ty = *e.weight();
        if child == virtual_root {
          continue;
        }

        let Some(&child_key) = self.bundle_root_graph.node_weight(child) else {
          continue;
        };

        let child_bundle_id = IdealBundleId::from_asset_key(child_key);
        let Some(child_bundle) = ideal.get_bundle(&child_bundle_id) else {
          continue;
        };

        // Isolated bundles start fresh.
        if child_bundle.behavior == Some(BundleBehavior::Isolated)
          || child_bundle.behavior == Some(BundleBehavior::InlineIsolated)
          || child_bundle.behavior == Some(BundleBehavior::Inline)
        {
          ancestor_by_node.insert(child, RoaringBitmap::new());
          continue;
        }

        let current_child_available = if edge_ty == BundleRootEdgeType::Parallel {
          let mut tmp = available.clone();
          tmp |= &parallel_availability;
          tmp
        } else {
          available.clone()
        };

        // Intersect across all parents.
        if let Some(existing) = ancestor_by_node.get_mut(&child) {
          *existing &= &current_child_available;
        } else {
          ancestor_by_node.insert(child, current_child_available.clone());
        }

        // Update parallel availability for later siblings.
        if edge_ty == BundleRootEdgeType::Parallel {
          // Later siblings should see earlier sibling's reachable assets.
          // Use the child's full sync-reachable set (JS `reachableAssets[child]`).
          if let Some(&child_bit) = reachability.root_to_bit.get(&child_key)
            && let Some(reachable) = reachable_assets_per_root.get(child_bit)
          {
            parallel_availability |= reachable;
          }
          parallel_availability.insert(child_key.0);
        }
      }
    }

    debug!(
      bundles = ideal.bundle_count(),
      "ideal graph: computed availability (bundle_root_graph)"
    );

    Ok(())
  }

  // ----------------------------
  // Phase 6: Reachability
  // ----------------------------

  /// Derive reachability by propagating bundle-root bitsets through the sync graph in
  /// topological order.
  ///
  /// Returns reachability as bitsets keyed by sync-graph node index.
  ///
  /// If the sync graph is cyclic, this will return an error and the caller may fall back to
  /// another reachability strategy.
  #[instrument(level = "debug", skip_all)]
  fn compute_reachability_topological(&self) -> anyhow::Result<Reachability> {
    let virtual_root = self
      .virtual_root
      .context("sync graph virtual root not initialized")?;

    // Deterministic root ordering.
    let mut roots: Vec<AssetKey> = self.bundle_roots.iter().copied().collect();
    roots.sort();

    let num_roots = roots.len();

    // StableGraph indices are not dense; use `node_bound` and index via `NodeIndex::index()`.
    let bound = self.sync_graph.node_bound();

    if num_roots == 0 {
      let root_to_bit: HashMap<AssetKey, usize> = HashMap::new();
      return Ok(Reachability {
        reach_bits: vec![FixedBitSet::with_capacity(0); bound],
        roots,
        root_to_bit,
      });
    }

    let mut reach_bits: Vec<FixedBitSet> = vec![FixedBitSet::with_capacity(num_roots); bound];

    // Seed root bits.
    // Track the initial seed bits separately so we can stop propagation past bundleBehavior assets.
    let mut initial_bits: HashMap<NodeIndex, FixedBitSet> = HashMap::new();
    for (i, root_key) in roots.iter().copied().enumerate() {
      let Some(&root_idx) = self.asset_to_sync_node.get(&root_key) else {
        continue;
      };

      reach_bits[root_idx.index()].insert(i);

      let mut bs = FixedBitSet::with_capacity(num_roots);
      bs.insert(i);
      initial_bits.insert(root_idx, bs);
    }

    let root_to_bit: HashMap<AssetKey, usize> =
      roots.iter().enumerate().map(|(i, &k)| (k, i)).collect();

    if let Ok(order) = petgraph::algo::toposort(&self.sync_graph, None) {
      // ----------------------------
      // Fast path: DAG topo propagation
      // ----------------------------
      for node in order {
        if node == virtual_root {
          continue;
        }

        let bits = reach_bits[node.index()].clone();
        if bits.is_empty() {
          continue;
        }

        // JS semantics: `asset.bundleBehavior != null` stops OTHER roots' reachability
        // traversal (skipChildren). The bundleBehavior asset's OWN root bit still propagates
        // to its children (it IS a bundle root with its own seeded bit).
        let propagate_bits = if let Some(asset_key) = self.bundle_behavior_sync_nodes.get(&node) {
          // Only propagate this node's own root bit, not inherited upstream bits.
          let mut own_bits = FixedBitSet::with_capacity(num_roots);
          if let Some(&root_bit) = root_to_bit.get(asset_key) {
            if bits.contains(root_bit) {
              own_bits.insert(root_bit);
            }
          }
          own_bits
        } else {
          bits
        };

        if propagate_bits.is_empty() {
          continue;
        }

        for succ in self
          .sync_graph
          .neighbors_directed(node, Direction::Outgoing)
        {
          reach_bits[succ.index()].union_with(&propagate_bits);
        }
      }
    } else {
      // ----------------------------
      // Cycle-handling path: SCC condensation
      // ----------------------------
      debug!("ideal graph: sync graph has cycles; computing reachability via SCC condensation");

      let sccs: Vec<Vec<NodeIndex>> = kosaraju_scc(&self.sync_graph);

      let mut scc_of: HashMap<NodeIndex, usize> = HashMap::new();
      for (i, scc) in sccs.iter().enumerate() {
        for &n in scc {
          scc_of.insert(n, i);
        }
      }

      // Build SCC DAG.
      let mut scc_graph: StableDiGraph<usize, ()> = StableDiGraph::new();
      let scc_nodes: Vec<NodeIndex> = (0..sccs.len()).map(|i| scc_graph.add_node(i)).collect();

      for e in self.sync_graph.edge_references() {
        let a = e.source();
        let b = e.target();
        let sa = scc_of[&a];
        let sb = scc_of[&b];
        if sa != sb {
          scc_graph.add_edge(scc_nodes[sa], scc_nodes[sb], ());
        }
      }

      let scc_order = petgraph::algo::toposort(&scc_graph, None).map_err(|cycle| {
        anyhow::anyhow!(
          "SCC condensation graph unexpectedly cyclic (cycle at {:?})",
          cycle.node_id()
        )
      })?;

      // Aggregate initial bits per SCC (union of member seeds).
      let mut scc_bits: Vec<FixedBitSet> = vec![FixedBitSet::with_capacity(num_roots); sccs.len()];

      for (scc_idx, nodes) in sccs.iter().enumerate() {
        let mut agg = FixedBitSet::with_capacity(num_roots);

        for &n in nodes {
          agg.union_with(&reach_bits[n.index()]);
        }

        scc_bits[scc_idx] = agg;
      }

      // Build propagation mask per SCC: if any member is a bundleBehavior sync node,
      // only propagate that member's own root bit (matching JS skipChildren).
      let mut scc_propagate_mask: Vec<Option<FixedBitSet>> = vec![None; sccs.len()];
      for (scc_idx, nodes) in sccs.iter().enumerate() {
        let has_bundle_behavior_node = nodes
          .iter()
          .any(|&n| self.bundle_behavior_sync_nodes.contains_key(&n));

        if has_bundle_behavior_node {
          // Only propagate own root bits of bundleBehavior members in this SCC.
          let mut own_bits = FixedBitSet::with_capacity(num_roots);
          for &n in nodes {
            if let Some(asset_key) = self.bundle_behavior_sync_nodes.get(&n) {
              if let Some(&root_bit) = root_to_bit.get(asset_key) {
                own_bits.insert(root_bit);
              }
            }
          }
          scc_propagate_mask[scc_idx] = Some(own_bits);
        }
      }

      // Propagate across SCC DAG.
      for scc_node in scc_order {
        let scc_idx = *scc_graph
          .node_weight(scc_node)
          .context("SCC node missing")?;

        let bits = scc_bits[scc_idx].clone();
        if bits.is_empty() {
          continue;
        }

        // Apply bundleBehavior mask: only propagate own root bits from this SCC.
        let propagate_bits = if let Some(mask) = &scc_propagate_mask[scc_idx] {
          let mut masked = bits;
          masked.intersect_with(mask);
          masked
        } else {
          bits
        };

        if propagate_bits.is_empty() {
          continue;
        }

        for succ in scc_graph.neighbors_directed(scc_node, Direction::Outgoing) {
          let succ_idx = *scc_graph
            .node_weight(succ)
            .context("SCC successor missing")?;
          scc_bits[succ_idx].union_with(&propagate_bits);
        }
      }

      // Assign SCC bits back to nodes.
      for (scc_idx, nodes) in sccs.iter().enumerate() {
        for &n in nodes {
          reach_bits[n.index()] = scc_bits[scc_idx].clone();
        }
      }
    }

    // Count assets with non-trivial reachability for decision logging parity.
    let mut assets_with_reachability = 0usize;
    for (&asset_key, &idx) in &self.asset_to_sync_node {
      if idx == virtual_root {
        continue;
      }

      let bs = &reach_bits[idx.index()];
      if bs.is_empty() {
        continue;
      }

      // Skip bundle roots unless they are reachable from other bundle roots.
      if self.bundle_roots.contains(&asset_key)
        && bs.count_ones(..) == 1
        && bs
          .ones()
          .next()
          .is_some_and(|bit| roots.get(bit).is_some_and(|r| *r == asset_key))
      {
        continue;
      }

      assets_with_reachability += 1;
    }

    debug!(
      assets_with_reachability,
      "ideal graph: computed topo-based reachability"
    );

    Ok(Reachability {
      reach_bits,
      roots,
      root_to_bit,
    })
  }

  // ----------------------------
  // Phase 6: Single-root asset placement
  // ----------------------------

  /// Place assets that are reachable from exactly one bundle root into that root's bundle.
  ///
  /// This covers:
  /// - Assets dominated by a single non-entry root (dominator subtree).
  /// - Assets reachable from a single entry root only.
  /// - Assets reachable from multiple roots but where all are entry roots (placed in first entry).
  ///
  /// Multi-root assets (reachable from >1 non-entry roots) are left unplaced for Phase 8.
  #[instrument(level = "debug", skip_all)]
  fn place_single_root_assets(
    &mut self,
    reachability: &Reachability,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Precompute bundle ids for all known bundle roots once.
    //
    // This avoids repeated `self.assets.id_for(root).to_string()` allocations in the hot loop.
    let root_bundle_ids: HashMap<AssetKey, IdealBundleId> = self
      .bundle_roots
      .iter()
      .chain(self.entry_like_roots.iter())
      .map(|&key| (key, IdealBundleId::from_asset_key(key)))
      .collect();

    // Type-change sibling bundles: keyed by (parent_bundle_root, file_type_str).
    // When placing a CSS asset into a JS bundle, redirect into a sibling bundle.
    // This matches the JS bundler's `addAssetToBundleRoot` which uses
    // `bundleGroup.mainEntryAsset.id + '.' + asset.type` as a coalescing key.
    self.type_change_siblings.clear();

    let sync_nodes: Vec<(AssetKey, NodeIndex)> = self
      .asset_to_sync_node
      .iter()
      .map(|(&asset_key, &node_idx)| (asset_key, node_idx))
      .collect();

    for (asset_key, node_idx) in sync_nodes {
      let bs = &reachability.reach_bits[node_idx.index()];
      if bs.is_empty() {
        continue;
      }

      // Entry-like roots: entries, non-splittable, isolated, needs-stable-name.
      // These get assets duplicated into them (same as entries in JS algorithm).
      // Note: we must compute this even for bundle roots, so that bundle roots can be
      // duplicated into entry-like bundles (matching the JS bundler).
      let reaching_entry_like: Vec<AssetKey> = bs
        .ones()
        .map(|i| reachability.roots[i])
        .filter(|r| self.entry_like_roots.contains(r))
        .collect();

      let splittable_roots: Vec<AssetKey> = bs
        .ones()
        .map(|i| reachability.roots[i])
        .filter(|r| !self.entry_like_roots.contains(r))
        .collect();

      // Availability filtering (JS Insert Or Share semantics):
      // Only splittable roots are filtered by availability. Entry-like roots ALWAYS
      // get assets placed into them regardless of availability (matching JS
      // addAssetToBundleRoot which is called unconditionally for entry-like roots).
      let available_roots: HashSet<AssetKey> = splittable_roots
        .iter()
        .copied()
        .filter(|root| {
          root_bundle_ids
            .get(root)
            .and_then(|id| ideal.get_bundle(id))
            .is_some_and(|b| b.ancestor_assets.contains(asset_key.0))
        })
        .collect();

      // Note: we do NOT skip assets where all splittable roots have them "available".
      // Parallel sibling availability can be circular (A sees B's assets as available,
      // B sees A's), which would cause assets to be skipped entirely. Instead, we let
      // the eligible_splittable_roots filtering handle this naturally — if all roots
      // are filtered, the co-load fallback places the asset in the first root.

      // Duplicate asset into ALL reaching entry-like bundles (matching JS algorithm).
      // Each entry-like bundle must independently contain every sync-reachable asset
      // because they can be loaded in isolation.
      //
      // If the asset's file type differs from the bundle's type (e.g. CSS asset in JS bundle),
      // redirect it into a type-change sibling bundle keyed by (root, file_type).
      for &root in &reaching_entry_like {
        // Entry-like roots ALWAYS get assets placed — no availability check.
        // This matches JS addAssetToBundleRoot which is unconditional for entries.
        let bundle_id = &root_bundle_ids[&root];

        // Skip duplicating bundle roots into entry-like parents when a type change
        // would occur (e.g. inline CSS bundle root reached from an HTML entry).
        // These assets already have their own bundle and duplicating them into a
        // type-change sibling creates extra bundles that the JS bundler doesn't produce.
        if asset_key != root && self.bundle_roots.contains(&asset_key) {
          let asset_ft = self.asset_file_types.get(&asset_key);
          let parent_bundle = ideal.get_bundle(bundle_id);
          if let (Some(aft), Some(pb)) = (asset_ft, parent_bundle) {
            if *aft != pb.bundle_type {
              continue;
            }
          }
        }

        let target_bundle_id = if asset_key == root {
          *bundle_id
        } else {
          self.resolve_type_change_target(asset_key, root, bundle_id, ideal)?
        };

        let bundle = ideal
          .get_bundle_mut(&target_bundle_id)
          .context("bundle missing for entry-like duplication")?;
        bundle.assets.insert(asset_key.0 as usize);

        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset: asset_key,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::SingleEligibleRoot,
          },
        );
      }

      // Track canonical bundle assignment (smallest *placed* entry-like for determinism).
      // If all entry-like roots already have the asset available, we skip canonical assignment here.
      if let Some(canonical) = reaching_entry_like
        .iter()
        .copied()
        .filter(|r| !available_roots.contains(r))
        .min()
      {
        let parent_bundle_id = root_bundle_ids[&canonical];
        let canonical_bundle_id = if asset_key == canonical {
          parent_bundle_id
        } else {
          self.resolve_type_change_target(asset_key, canonical, &parent_bundle_id, ideal)?
        };
        // Only record canonical placement here (asset may have been duplicated into multiple bundles).
        // `move_asset_to_bundle` would remove it from the other entry-like bundles, which we don't want.
        ideal.set_asset_bundle(asset_key, Some(canonical_bundle_id));
      }

      // Bundle roots have their own bundles — skip splittable/shared placement.
      // (But keep the entry-like duplication + canonical assignment above.)
      if self.bundle_roots.contains(&asset_key) {
        continue;
      }

      // If this asset is reachable from an entry root, do not place it into splittable
      // (async) roots as well. Entry bundles are ancestors of async bundles, so the asset
      // will be available via ancestor_assets (Phase 8a).
      //
      // Special-case: `esmodule-helpers.js` should be treated as entry-like when also reachable
      // from any entry-like root. This prevents it from being placed into async bundles in
      // scenarios where it is also reachable from an entry-like bundle.
      if reaching_entry_like
        .iter()
        .any(|r| self.entry_roots.contains(r))
      {
        continue;
      }

      // After availability filtering, some roots may be removed because the asset is already
      // available via ancestor bundles. If that reduces the eligible splittable roots to 1,
      // JS will insert the asset into that single remaining root rather than creating a
      // shared bundle.
      let eligible_splittable_roots: Vec<AssetKey> = splittable_roots
        .iter()
        .copied()
        .filter(|r| !available_roots.contains(r))
        .collect();

      if eligible_splittable_roots.len() > 1 {
        // Multi-root asset with no entry-like roots -> deferred to Phase 8/9 (shared bundles).
        continue;
      } else if eligible_splittable_roots.is_empty() && !splittable_roots.is_empty() {
        // All splittable roots filtered by availability (co-load).
        // The asset is available because of bundle-group co-load semantics:
        // parallel siblings make their reachable assets available to each other.
        // JS places assets during Phase 3 DFS before availability is computed, so
        // they remain in the first root's bundle. We must replicate that placement.
        let root = splittable_roots[0];

        let bundle_id = &root_bundle_ids[&root];
        let target_bundle_id =
          self.resolve_type_change_target(asset_key, root, bundle_id, ideal)?;

        let bundle = ideal
          .get_bundle_mut(&target_bundle_id)
          .context("bundle missing for co-load available placement")?;
        bundle.assets.insert(asset_key.0 as usize);

        if reaching_entry_like.is_empty() {
          ideal.move_asset_to_bundle(asset_key, &target_bundle_id)?;
        }

        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset: asset_key,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::DominatorSubtree,
          },
        );
      } else if eligible_splittable_roots.len() == 1 {
        // Single eligible splittable root after availability filtering -> place directly.
        // If file types differ, redirect to a type-change sibling bundle.
        let root = eligible_splittable_roots[0];

        let bundle_id = &root_bundle_ids[&root];
        let target_bundle_id =
          self.resolve_type_change_target(asset_key, root, bundle_id, ideal)?;

        let bundle = ideal
          .get_bundle_mut(&target_bundle_id)
          .context("bundle missing for single-root placement")?;
        bundle.assets.insert(asset_key.0 as usize);

        // If this asset was duplicated into one or more entry-like bundles, we must not
        // "move" it here (which would remove it from the canonical entry-like bundle).
        // Keep the canonical `asset_to_bundle` mapping stable in that case.
        if reaching_entry_like.is_empty() {
          ideal.move_asset_to_bundle(asset_key, &target_bundle_id)?;
        }

        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset: asset_key,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::DominatorSubtree,
          },
        );
      }
    }

    debug!(
      placed = ideal.asset_to_bundle.iter().filter(|b| b.is_some()).count(),
      "ideal graph: placed single-root assets"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 6 helper: type-change sibling bundle resolution
  // ----------------------------

  /// If the asset's file type differs from the target bundle's type, resolve to a
  /// type-change sibling bundle keyed by `(parent_root, file_type)`.
  ///
  /// This matches the JS bundler's `addAssetToBundleRoot` which uses
  /// `bundleGroup.mainEntryAsset.id + '.' + asset.type` to coalesce all CSS assets
  /// from the same parent bundle group into a single CSS sibling bundle.
  ///
  /// Returns the original `parent_bundle_id` if no type change, or the sibling bundle id.
  fn resolve_type_change_target(
    &mut self,
    asset_key: AssetKey,
    parent_root: AssetKey,
    parent_bundle_id: &IdealBundleId,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<IdealBundleId> {
    let asset_file_type = self.asset_file_types.get(&asset_key);
    let parent_bundle = ideal
      .get_bundle(parent_bundle_id)
      .context("parent bundle missing in resolve_type_change_target")?;
    let parent_type = &parent_bundle.bundle_type;

    // No type change -> place directly in parent bundle.
    if asset_file_type.is_none() || asset_file_type == Some(parent_type) {
      return Ok(*parent_bundle_id);
    }

    // Bundle roots that would trigger a type change should not be redirected into
    // type-change sibling bundles. They already have their own bundle created in
    // Phase 3 (create_bundle_roots). Redirecting them creates extra empty-ish bundles
    // that the JS bundler does not produce (e.g. inline CSS from HTML <style> tags).
    if self.bundle_roots.contains(&asset_key) {
      return Ok(*parent_bundle_id);
    }

    let file_type = asset_file_type.unwrap().clone();
    let file_type_str = format!("{file_type:?}");
    let sibling_key = (parent_root, file_type_str.clone());

    if let Some(&existing_id) = self.type_change_siblings.get(&sibling_key) {
      return Ok(existing_id);
    }

    // Create a new type-change sibling bundle.
    let sibling_id_str = format!(
      "@@typechange:{}:{}",
      self.assets.id_for(parent_root),
      file_type_str
    );
    // Insert synthetic id into the asset interner so IdealBundleId works.
    let sibling_asset_key = ideal.assets.insert_synthetic(sibling_id_str);
    let sibling_bundle_id = IdealBundleId::from_asset_key(sibling_asset_key);

    ideal.create_bundle(IdealBundle {
      id: sibling_bundle_id,
      root_asset_id: None, // Type-change sibling bundles have no root asset.
      bundle_group_root: None,
      assets: FixedBitSet::with_capacity(self.assets.len()),
      bundle_type: file_type,
      needs_stable_name: false,
      behavior: None,
      ancestor_assets: RoaringBitmap::new(),
    })?;

    // Add a sync edge from the parent bundle to the sibling.
    ideal.add_bundle_edge(*parent_bundle_id, sibling_bundle_id, IdealEdgeType::Sync);

    self
      .type_change_siblings
      .insert(sibling_key, sibling_bundle_id);

    Ok(sibling_bundle_id)
  }

  // ----------------------------
  // Phase 9: Shared bundle creation
  // ----------------------------

  /// Extract shared bundles for multi-root assets (reachable from >1 non-entry roots).
  ///
  /// After availability is computed over placed single-root assets (Phase 8a),
  /// we filter multi-root assets by availability and either:
  /// - Place into a single eligible root (if availability reduces to 1).
  /// - Extract into a shared bundle.
  #[instrument(level = "debug", skip_all)]
  fn create_shared_bundles(
    &mut self,
    reachability: &Reachability,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Match JS bundler default: only create a shared bundle if an asset is reachable
    // from more than 1 eligible root.

    // Precompute bundle ids for all known bundle roots once.
    //
    // Include reachability roots because some may have been removed from `bundle_roots`
    // during internalization but still appear in the reachability data.
    let root_bundle_ids: HashMap<AssetKey, IdealBundleId> = self
      .bundle_roots
      .iter()
      .chain(self.entry_like_roots.iter())
      .chain(reachability.roots.iter())
      .map(|&key| (key, IdealBundleId::from_asset_key(key)))
      .collect();

    let mut eligible_roots_by_asset: HashMap<AssetKey, Vec<AssetKey>> = HashMap::new();

    let sync_nodes: Vec<(AssetKey, NodeIndex)> = self
      .asset_to_sync_node
      .iter()
      .map(|(&asset_key, &node_idx)| (asset_key, node_idx))
      .collect();

    for (asset_key, node_idx) in sync_nodes {
      let bs = &reachability.reach_bits[node_idx.index()];
      if bs.is_empty() {
        continue;
      }

      // Bundle roots are already handled as bundles (unless they were internalized and removed
      // from `bundle_roots`, in which case they'll be treated as regular assets here).
      if self.bundle_roots.contains(&asset_key) {
        continue;
      }

      // Track whether this asset is also reachable from any entry-like root.
      // If so, we must avoid `move_asset_to_bundle` later because that would remove it
      // from the entry-like bundle (duplication semantics).
      let reaching_entry_like: Vec<AssetKey> = bs
        .ones()
        .map(|i| reachability.roots[i])
        .filter(|r| self.entry_like_roots.contains(r))
        .collect();

      // Compute reachable splittable roots.
      let mut reachable: Vec<AssetKey> = bs
        .ones()
        .map(|i| reachability.roots[i])
        .filter(|r| !self.entry_like_roots.contains(r))
        .collect();

      reachable.sort();
      reachable.dedup();

      // Availability filtering (JS Insert Or Share semantics):
      // roots where the asset is already available via `ancestor_assets[root]` are removed.
      let mut eligible: Vec<AssetKey> = reachable
        .iter()
        .copied()
        .filter(|root| {
          root_bundle_ids
            .get(root)
            .and_then(|id| ideal.get_bundle(id))
            .is_none_or(|b| !b.ancestor_assets.contains(asset_key.0))
        })
        .collect();

      eligible.sort();
      eligible.dedup();

      // Subgraph reuse filtering (matches JS Insert Or Share "subgraph absorption"):
      // If an eligible root A can sync-reach another eligible root B, then A doesn't
      // need to participate in a shared bundle for this asset. A can instead reach the
      // asset via B's bundle, so we remove A from the eligible set and add a bundle
      // edge from A -> B.
      {
        let mut to_remove: HashSet<AssetKey> = HashSet::new();

        // Pass 1: If the current asset is itself a bundle root, reuse its bundle directly.
        let asset_bundle_id = IdealBundleId::from_asset_key(asset_key);
        if ideal.get_bundle(&asset_bundle_id).is_some() {
          for &candidate in &eligible {
            to_remove.insert(candidate);
            ideal.add_bundle_edge(
              IdealBundleId::from_asset_key(candidate),
              asset_bundle_id,
              IdealEdgeType::Sync,
            );
          }
        } else {
          // Pass 2: Inter-root reachability filtering.
          for &candidate in &eligible {
            if to_remove.contains(&candidate) {
              continue;
            }

            let Some(&candidate_bit) = reachability.root_to_bit.get(&candidate) else {
              continue;
            };

            for &other in &eligible {
              if other == candidate || to_remove.contains(&other) {
                continue;
              }

              let Some(&other_sync_idx) = self.asset_to_sync_node.get(&other) else {
                continue;
              };

              // `reach_bits[node]` contains bits for all roots that can reach `node`.
              // If `candidate`'s bit is set on `other`'s sync node, then `candidate`
              // can sync-reach `other` (i.e. `other` appears in `reachableAssets[candidate]` in JS).
              if reachability.reach_bits[other_sync_idx.index()].contains(candidate_bit) {
                to_remove.insert(candidate);
                ideal.add_bundle_edge(
                  IdealBundleId::from_asset_key(candidate),
                  IdealBundleId::from_asset_key(other),
                  IdealEdgeType::Sync,
                );
                break;
              }
            }
          }
        }

        if !to_remove.is_empty() {
          eligible.retain(|r| !to_remove.contains(r));
        }
      }

      match eligible.len() {
        0 => {
          // All splittable roots have this asset available via ancestors.
          // If the asset was already placed in entry-like bundles, it's truly handled upstream.
          // Otherwise, place it in the first reachable root as a co-load fallback to avoid
          // leaving assets unplaced (parallel sibling availability can be circular).
          if reaching_entry_like.is_empty() && !reachable.is_empty() {
            let root = reachable[0];
            let bundle_id = &root_bundle_ids[&root];
            let target_bundle_id =
              self.resolve_type_change_target(asset_key, root, bundle_id, ideal)?;
            let bundle = ideal
              .get_bundle_mut(&target_bundle_id)
              .context("bundle missing for co-load fallback in create_shared_bundles")?;
            bundle.assets.insert(asset_key.0 as usize);
            ideal.move_asset_to_bundle(asset_key, &target_bundle_id)?;
          }
          continue;
        }
        1 => {
          // JS: if filtering reduces to a single eligible root, insert into that root.
          //
          // Direct reuse (matches JS Insert Or Share): if the current asset is itself a bundle
          // root (it has its own bundle), don't move it. Instead, connect the eligible root's
          // bundle to this existing bundle.
          let asset_bundle_id = IdealBundleId::from_asset_key(asset_key);
          if ideal.get_bundle(&asset_bundle_id).is_some() {
            let from_root = eligible[0];
            let from_id = root_bundle_ids[&from_root];
            if from_id != asset_bundle_id {
              ideal.add_bundle_edge(from_id, asset_bundle_id, IdealEdgeType::Sync);
            }
            continue;
          }

          let root = eligible[0];

          let bundle_id = &root_bundle_ids[&root];
          let target_bundle_id =
            self.resolve_type_change_target(asset_key, root, bundle_id, ideal)?;

          let bundle = ideal
            .get_bundle_mut(&target_bundle_id)
            .context("bundle missing for single-root placement in create_shared_bundles")?;
          bundle.assets.insert(asset_key.0 as usize);

          // If this asset was duplicated into one or more entry-like bundles, we must not
          // "move" it here (which would remove it from the canonical entry-like bundle).
          if reaching_entry_like.is_empty() {
            ideal.move_asset_to_bundle(asset_key, &target_bundle_id)?;
          }

          self.decision(
            "placement",
            super::types::DecisionKind::AssetAssignedToBundle {
              asset: asset_key,
              bundle_root: root,
              reason: super::types::AssetAssignmentReason::SingleEligibleRoot,
            },
          );

          continue;
        }
        _ => {}
      }

      // Eligible roots set becomes the grouping key.
      eligible_roots_by_asset.insert(asset_key, eligible);
    }

    if eligible_roots_by_asset.is_empty() {
      debug!("ideal graph: no shared bundle candidates");
      return Ok(());
    }

    // Group assets by eligible root set (so we create fewer shared bundles).
    //
    // IMPORTANT: also group by file type to avoid mixing JS and CSS (or other types)
    // in the same shared bundle.
    let mut assets_by_root_set_and_type: HashMap<(Vec<AssetKey>, String), Vec<AssetKey>> =
      HashMap::new();
    for (asset_key, roots) in eligible_roots_by_asset {
      let file_type = self
        .asset_file_types
        .get(&asset_key)
        .map(|ft| format!("{ft:?}"))
        .unwrap_or_default();
      assets_by_root_set_and_type
        .entry((roots, file_type))
        .or_default()
        .push(asset_key);
    }
    tracing::debug!(
      unique_root_sets = assets_by_root_set_and_type.len(),
      "create_shared_bundles: grouped assets by root set"
    );

    // Precompute entry-like bundle ids so we can preserve duplication semantics when
    // extracting shared bundles.
    let entry_like_bundle_ids: HashSet<IdealBundleId> = self
      .entry_like_roots
      .iter()
      .filter_map(|root| root_bundle_ids.get(root).copied())
      .collect();

    for ((mut roots, file_type), assets) in assets_by_root_set_and_type {
      roots.sort();
      roots.dedup();

      let shared_name = format!(
        "@@shared:{file_type}:{}",
        roots
          .iter()
          .map(|r| self.assets.id_for(*r))
          .collect::<Vec<_>>()
          .join(",")
      );

      let shared_key = ideal.assets.insert_synthetic(shared_name);

      let shared_bundle_id = IdealBundleId::from_asset_key(shared_key);

      // Derive bundle type from the first asset in the group.
      let bundle_type = assets
        .first()
        .and_then(|a| self.asset_file_types.get(a).cloned())
        .unwrap_or(atlaspack_core::types::FileType::Js);

      ideal.create_bundle(super::types::IdealBundle {
        id: shared_bundle_id,
        root_asset_id: None,
        bundle_group_root: None,
        assets: FixedBitSet::with_capacity(self.assets.len()),
        bundle_type,
        needs_stable_name: false,
        behavior: None,
        ancestor_assets: RoaringBitmap::new(),
      })?;

      self.decision(
        "shared",
        super::types::DecisionKind::SharedBundleCreated {
          shared_bundle_root: shared_key,
          source_bundle_roots: roots.clone(),
          asset_count: assets.len(),
        },
      );

      for asset in assets {
        let prev = ideal.asset_bundle(&asset);
        let was_in_entry_like = prev.is_some_and(|pid| entry_like_bundle_ids.contains(&pid));

        if was_in_entry_like {
          // Entry-like bundles (entries, isolated, non-splittable) duplicate assets into them.
          // When extracting a shared bundle we must NOT remove the asset from those bundles,
          // but we DO want the shared bundle to become the canonical assignment.
          let bundle = ideal
            .get_bundle_mut(&shared_bundle_id)
            .context("shared bundle missing")?;
          bundle.assets.insert(asset.0 as usize);
          ideal.set_asset_bundle(asset, Some(shared_bundle_id));
        } else {
          // Normal case: move asset out of its previous canonical bundle.
          ideal.move_asset_to_bundle(asset, &shared_bundle_id)?;
        }

        if let Some(&from_root) = roots.first() {
          self.decision(
            "shared",
            super::types::DecisionKind::AssetMovedToSharedBundle {
              asset,
              from_bundle_root: from_root,
              shared_bundle_root: shared_key,
            },
          );
        }
      }

      // Sync edges from each source root to the shared bundle.
      for root in roots {
        let from_id = root_bundle_ids[&root];
        ideal.add_bundle_edge(from_id, shared_bundle_id, IdealEdgeType::Sync);
      }
    }

    debug!(
      bundles = ideal.bundle_count(),
      "ideal graph: shared bundles created"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 8: Internalization
  // ----------------------------

  /// Internalize async bundles whose root asset is already available from all parent bundles.
  ///
  /// An async bundle is redundant if every parent bundle that references it already has the
  /// bundle root asset available (either directly in the bundle or via ancestor availability).
  /// In that case, the async bundle is deleted -- the parent will resolve the import internally.
  ///
  /// Matches the JS algorithm's "Step Internalize async bundles" behavior.
  fn internalize_async_bundles(
    &mut self,
    _asset_graph: &AssetGraph,
    reachability: &mut Reachability,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    let Some(virtual_root) = self.bundle_root_graph_virtual_root else {
      return Ok(());
    };

    // Mirrors JS idealGraph.ts internalization (lines 858-900):
    // For each non-entry bundleRootGraph node, check if ALL parents have the
    // child root available (sync-reachable or in ancestor_assets). If yes,
    // the bundle can be deleted (internalized).
    let mut to_internalize: Vec<(IdealBundleId, Vec<IdealBundleId>, AssetKey)> = Vec::new();

    for node in self.bundle_root_graph.node_indices().collect::<Vec<_>>() {
      if node == virtual_root {
        continue;
      }

      let Some(&root_key) = self.bundle_root_graph.node_weight(node) else {
        continue;
      };

      // Skip entries (JS: entries are never internalized)
      if self.entry_roots.contains(&root_key) {
        continue;
      }

      let bundle_id = IdealBundleId::from_asset_key(root_key);

      let Some(bundle) = ideal.get_bundle(&bundle_id) else {
        continue;
      };

      // JS: canDelete starts as (behavior !== 'isolated' && behavior !== 'inlineIsolated' && behavior !== 'inline')
      let mut can_delete = bundle.behavior != Some(BundleBehavior::Isolated)
        && bundle.behavior != Some(BundleBehavior::InlineIsolated)
        && bundle.behavior != Some(BundleBehavior::Inline);

      // Get all parents via incoming edges (JS: bundleRootGraph.getNodeIdsConnectedTo)
      let parent_nodes: Vec<NodeIndex> = self
        .bundle_root_graph
        .edges_directed(node, Direction::Incoming)
        .map(|e| e.source())
        .collect();

      if parent_nodes.is_empty() {
        continue;
      }

      let mut parent_bundle_ids: Vec<IdealBundleId> = Vec::new();

      for &parent_node in &parent_nodes {
        // JS: if (parentId === rootNodeId) { canDelete = false; continue; }
        if parent_node == virtual_root {
          can_delete = false;
          continue;
        }

        let Some(&parent_key) = self.bundle_root_graph.node_weight(parent_node) else {
          can_delete = false;
          continue;
        };

        // JS: reachableAssets[parentId].has(bundleRootId) || ancestorAssets[parentId]?.has(bundleRootId)
        let parent_bundle_id = IdealBundleId::from_asset_key(parent_key);

        // Check sync-reachability: can parent root sync-reach child root?
        let sync_reachable = if let Some(&p_bit) = reachability.root_to_bit.get(&parent_key)
          && let Some(&c_sync_node) = self.asset_to_sync_node.get(&root_key)
        {
          reachability.reach_bits[c_sync_node.index()].contains(p_bit)
        } else {
          false
        };

        // Check ancestor_assets
        let in_ancestors = ideal
          .get_bundle(&parent_bundle_id)
          .is_some_and(|b| b.ancestor_assets.contains(root_key.0));

        if sync_reachable || in_ancestors {
          // JS: parentBundle.internalizedAssets.add(bundleRootId)
          parent_bundle_ids.push(parent_bundle_id);
        } else {
          can_delete = false;
        }
      }

      if can_delete {
        parent_bundle_ids.sort();
        parent_bundle_ids.dedup();
        to_internalize.push((bundle_id, parent_bundle_ids, root_key));

        self.decision(
          "internalization",
          super::types::DecisionKind::BundleInternalized {
            bundle_root: root_key,
          },
        );
      }
    }

    // Apply internalization: match JS `deleteBundle` behavior.
    //
    // The JS bundler deletes the async bundle entirely. Keeping an "empty shell" bundle
    // causes downstream materialization to treat the root asset as a bundle boundary and
    // stop sync traversal early, which can make the internalized root asset disappear from
    // all emitted bundles.
    for (bundle_id, parents, root_key) in &to_internalize {
      // Remove from bundle_roots so placement/shared phases treat the root as a normal asset.
      self.bundle_roots.remove(root_key);

      // Track as internalized (used later during NativeBundleGraph materialization).
      ideal
        .internalized_bundles
        .insert(*bundle_id, parents.clone());

      // Delete the bundle (removes edges and clears `asset_to_bundle` for its assets).
      ideal.remove_bundle(bundle_id);
    }

    // Clear internalized root bits from reachability bitsets.
    // This matches JS behavior where deleted bundleRootGraph nodes are
    // skipped in Insert Or Share, preventing assets only reachable from
    // internalized roots from being placed.
    for (_bundle_id, _parents, root_key) in &to_internalize {
      if let Some(&bit) = reachability.root_to_bit.get(root_key) {
        for bs in reachability.reach_bits.iter_mut() {
          bs.set(bit, false);
        }
      }
    }

    if !to_internalize.is_empty() {
      debug!(
        internalized = to_internalize.len(),
        "ideal graph: internalized bundle roots"
      );
    }

    Ok(())
  }

  // ----------------------------
  // Helpers
  // ----------------------------

  fn asset_node_ids(&self, asset_graph: &AssetGraph) -> Vec<NodeId> {
    // We don't have direct access to internal indices, so we resolve from content-key map.
    asset_graph
      .get_assets()
      .filter_map(|a| asset_graph.get_node_id_by_content_key(&a.id).copied())
      .collect()
  }
}
