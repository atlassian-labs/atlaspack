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
  asset_graph::{AssetGraph, NodeId},
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

  // Graph of bundle root assets with Lazy/Parallel edges.
  // Includes a virtual root connected to all entry bundle roots.
  bundle_root_graph: DiGraph<AssetKey, BundleRootEdgeType>,
  bundle_root_graph_nodes: HashMap<AssetKey, NodeIndex>,
  bundle_root_graph_virtual_root: Option<NodeIndex>,

  // Entry roots.
  entry_roots: HashSet<super::types::AssetKey>,

  // Bundle roots are either entry assets or boundary assets.
  bundle_roots: HashSet<super::types::AssetKey>,

  // Asset key -> file type, populated during asset interning.
  asset_file_types: HashMap<super::types::AssetKey, atlaspack_core::types::FileType>,

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

    // Phase 4: bundle dependency graph (edge types).
    self.build_bundle_edges(asset_graph, &mut ideal)?;

    // Phase 5: derive reachability via topological bitset propagation.
    let reachability = self.compute_reachability_topological()?;

    // Phase 5b: build bundle-root graph (lazy/parallel edges) for availability computation.
    self.build_bundle_root_graph(asset_graph)?;

    // Phase 6: availability propagation (ancestor_assets).
    //
    // Note: Phase 6 placement needs `ancestor_assets` to avoid placing assets that are already
    // available from an ancestor bundle (matches JS Insert Or Share filtering).
    self.compute_availability(&reachability, &mut ideal)?;

    // Phase 7: place single-root assets into their dominating bundle.
    self.place_single_root_assets(&reachability, &mut ideal)?;

    // Phase 8: internalize async bundles whose root is already available from all parents.
    self.internalize_async_bundles(asset_graph, &reachability, &mut ideal)?;

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

      for dep_node_id in asset_graph.get_outgoing_neighbors(&asset_node_id) {
        let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
          continue;
        };

        for target_asset_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
          let Some(target_asset) = asset_graph.get_asset(&target_asset_node_id) else {
            continue;
          };

          let dep_is_boundary = dep.priority != Priority::Sync;
          let type_change = from_asset.file_type != target_asset.file_type;
          let isolated = dep.bundle_behavior == Some(BundleBehavior::Isolated)
            || dep.bundle_behavior == Some(BundleBehavior::InlineIsolated)
            || from_asset.bundle_behavior == Some(BundleBehavior::Isolated)
            || from_asset.bundle_behavior == Some(BundleBehavior::InlineIsolated)
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
          let is_sync_type_change = type_change && !dep_is_boundary && !isolated;
          if (dep_is_boundary || type_change || isolated) && !is_sync_type_change {
            let Some(target_key) = self.assets.key_for(&target_asset.id) else {
              continue;
            };

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

    let virtual_root = self.sync_graph.add_node(SyncNode::VirtualRoot);
    self.virtual_root = Some(virtual_root);

    // Add all assets as nodes.
    for asset_id in self.assets.ids().iter() {
      let Some(key) = self.assets.key_for(asset_id) else {
        continue;
      };
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
      let Some(&from_idx) = self.asset_to_sync_node.get(&from_key) else {
        continue;
      };

      for dep_node_id in asset_graph.get_outgoing_neighbors(&asset_node_id) {
        let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
          continue;
        };

        if dep.priority != Priority::Sync {
          continue;
        }

        if dep.bundle_behavior == Some(BundleBehavior::Isolated)
          || dep.bundle_behavior == Some(BundleBehavior::InlineIsolated)
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
          if let Some(SyncNode::Asset(key)) = self.sync_graph.node_weight(node) {
            if let Some(&bundle_id) = map.get(key) {
              map.insert(asset_key, bundle_id);
              break;
            }
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
        }
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

    // For each root, walk its sync-reachable asset subgraph (using the sync-only graph).
    for root in roots {
      let Some(&from_node) = self.bundle_root_graph_nodes.get(&root) else {
        continue;
      };
      let Some(&root_sync_node) = self.asset_to_sync_node.get(&root) else {
        continue;
      };
      let Some((root_ctx, _root_behavior)) = asset_info.get(&root).copied() else {
        continue;
      };

      let mut queue: VecDeque<NodeIndex> = VecDeque::new();
      let mut visited: HashSet<NodeIndex> = HashSet::new();
      queue.push_back(root_sync_node);

      while let Some(n) = queue.pop_front() {
        if !visited.insert(n) {
          continue;
        }

        let Some(SyncNode::Asset(asset_key)) = self.sync_graph.node_weight(n).copied() else {
          continue;
        };

        // Skip traversing children past isolated assets (matches JS `asset.bundleBehavior != null`).
        if let Some((_ctx, Some(beh))) = asset_info.get(&asset_key) {
          if beh == &BundleBehavior::Isolated || beh == &BundleBehavior::InlineIsolated {
            continue;
          }
        }

        // Scan outgoing dependencies in the asset graph for non-sync edges that point to bundle roots.
        if let Some(asset_node_id) = asset_node_by_key.get(&asset_key) {
          for dep_node_id in asset_graph.get_outgoing_neighbors(asset_node_id) {
            let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
              continue;
            };

            // Only model lazy/parallel edges (conditional is not yet handled here).
            let edge_ty = match dep.priority {
              Priority::Lazy => Some(BundleRootEdgeType::Lazy),
              Priority::Parallel => Some(BundleRootEdgeType::Parallel),
              _ => None,
            };

            let Some(edge_ty) = edge_ty else {
              continue;
            };

            // Target assets of this dependency.
            for target_asset_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
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
              //
              // In JS, `bundle.bundleBehavior` is derived from the dependency or the target asset
              // and is `null` for normal splittable bundles. For our purposes, treat any explicit
              // bundle behavior on the dep or target asset as ineligible.
              if dep.bundle_behavior.is_some() || target_asset.bundle_behavior.is_some() {
                continue;
              }

              if env_is_isolated(target_asset.env.context) {
                continue;
              }

              if target_asset.env.context != root_ctx {
                continue;
              }

              let Some(&to_node) = self.bundle_root_graph_nodes.get(&target_key) else {
                continue;
              };

              self.bundle_root_graph.add_edge(from_node, to_node, edge_ty);
            }
          }
        }

        // Traverse to sync successors.
        for succ in self.sync_graph.neighbors_directed(n, Direction::Outgoing) {
          queue.push_back(succ);
        }
      }
    }

    debug!(
      bundle_root_nodes = self.bundle_root_graph.node_count(),
      bundle_root_edges = self.bundle_root_graph.edge_count(),
      "ideal graph: built bundle root graph"
    );

    Ok(())
  }

  // ----------------------------
  // Phase 6: Availability propagation
  // ----------------------------

  /// Computes `ancestor_assets` for each bundle root using the JS bundler semantics.
  ///
  /// This propagates availability down the `bundle_root_graph` (lazy/parallel edges) in
  /// topological order, intersecting availability through all parent paths.
  ///
  /// If the bundle-root graph is cyclic, we fall back to SCC condensation order.
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
    for entry in self.entry_roots.iter().copied() {
      if let Some(&idx) = self.bundle_root_graph_nodes.get(&entry) {
        ancestor_by_node.insert(idx, RoaringBitmap::new());
      }
    }

    // Determine processing order.
    let order: Vec<NodeIndex> = match petgraph::algo::toposort(&self.bundle_root_graph, None) {
      Ok(o) => o,
      Err(_) => {
        debug!(
          "ideal graph: bundle root graph has cycles; computing availability via SCC condensation"
        );
        self
          .bundle_root_graph_scc_order()
          .context("failed to compute SCC order for bundle root graph")?
      }
    };

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
      // NOTE: we intentionally skip the bundle-group co-load step for now.
      let mut available = if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
      {
        RoaringBitmap::new()
      } else {
        ancestor_by_node
          .get(&node)
          .cloned()
          .unwrap_or_else(RoaringBitmap::new)
      };

      // Union in this root's own reachable assets.
      //
      // JS uses `reachableAssets[root]` which is the full sync-reachable set from the bundle root.
      // In Rust we approximate this using the already-computed sync reachability bitsets: an asset
      // is reachable from this root if the asset's reachability set contains the root's bit.
      if let Some(&root_bit) = reachability.root_to_bit.get(&root_key) {
        for (&asset_key, &sync_idx) in &self.asset_to_sync_node {
          if reachability.reach_bits[sync_idx.index()].contains(root_bit) {
            available.insert(asset_key.0);
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

      // Deterministic order for children to match JS behavior.
      let mut children: Vec<(NodeIndex, BundleRootEdgeType)> = self
        .bundle_root_graph
        .edges_directed(node, Direction::Outgoing)
        .map(|e| (e.target(), *e.weight()))
        .collect();
      children.sort_by_key(|(child, ty)| {
        let w = self
          .bundle_root_graph
          .node_weight(*child)
          .copied()
          .unwrap_or(AssetKey(0));
        (
          w,
          match ty {
            BundleRootEdgeType::Parallel => 0u8,
            BundleRootEdgeType::Lazy => 1u8,
          },
        )
      });

      for (child, edge_ty) in children {
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
          if let Some(&child_bit) = reachability.root_to_bit.get(&child_key) {
            for (&asset_key, &sync_idx) in &self.asset_to_sync_node {
              if reachability.reach_bits[sync_idx.index()].contains(child_bit) {
                parallel_availability.insert(asset_key.0);
              }
            }
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

  /// Compute an ordering of `bundle_root_graph` nodes by SCC condensation (a DAG).
  ///
  /// This is a fallback for cycle handling when `petgraph::algo::toposort` fails.
  fn bundle_root_graph_scc_order(&self) -> anyhow::Result<Vec<NodeIndex>> {
    let sccs: Vec<Vec<NodeIndex>> = kosaraju_scc(&self.bundle_root_graph);

    let mut scc_of: HashMap<NodeIndex, usize> = HashMap::new();
    for (i, scc) in sccs.iter().enumerate() {
      for &n in scc {
        scc_of.insert(n, i);
      }
    }

    // Build SCC DAG.
    let mut scc_graph: StableDiGraph<usize, ()> = StableDiGraph::new();
    let scc_nodes: Vec<NodeIndex> = (0..sccs.len()).map(|i| scc_graph.add_node(i)).collect();

    for e in self.bundle_root_graph.edge_references() {
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
        "bundle_root_graph SCC condensation graph unexpectedly cyclic (cycle at {:?})",
        cycle.node_id()
      )
    })?;

    // Flatten SCCs in topo order, with deterministic ordering within SCC.
    let mut out: Vec<NodeIndex> = Vec::new();
    for scc_node in scc_order {
      let scc_idx = *scc_graph
        .node_weight(scc_node)
        .context("SCC node missing")?;

      let mut members = sccs[scc_idx].clone();
      members.sort_by_key(|n| {
        self
          .bundle_root_graph
          .node_weight(*n)
          .copied()
          .unwrap_or(AssetKey(0))
      });
      out.extend(members);
    }

    Ok(out)
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
    for (i, root_key) in roots.iter().copied().enumerate() {
      let Some(&root_idx) = self.asset_to_sync_node.get(&root_key) else {
        continue;
      };
      reach_bits[root_idx.index()].insert(i);
    }

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

        for succ in self
          .sync_graph
          .neighbors_directed(node, Direction::Outgoing)
        {
          reach_bits[succ.index()].union_with(&bits);
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

      // Propagate across SCC DAG.
      for scc_node in scc_order {
        let scc_idx = *scc_graph
          .node_weight(scc_node)
          .context("SCC node missing")?;

        let bits = scc_bits[scc_idx].clone();
        if bits.is_empty() {
          continue;
        }

        for succ in scc_graph.neighbors_directed(scc_node, Direction::Outgoing) {
          let succ_idx = *scc_graph
            .node_weight(succ)
            .context("SCC successor missing")?;
          scc_bits[succ_idx].union_with(&bits);
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

    let root_to_bit: HashMap<AssetKey, usize> =
      roots.iter().enumerate().map(|(i, &k)| (k, i)).collect();

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

      // Bundle roots are already placed.
      if self.bundle_roots.contains(&asset_key) {
        continue;
      }

      // Entry-like roots: entries, non-splittable, isolated, needs-stable-name.
      // These get assets duplicated into them (same as entries in JS algorithm).
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
      // If this asset is already available from *all* reaching roots (via ancestor bundles),
      // it doesn't need to be placed anywhere.
      let available_roots: HashSet<AssetKey> = reaching_entry_like
        .iter()
        .copied()
        .chain(splittable_roots.iter().copied())
        .filter(|root| {
          root_bundle_ids
            .get(root)
            .and_then(|id| ideal.get_bundle(id))
            .is_some_and(|b| b.ancestor_assets.contains(asset_key.0))
        })
        .collect();

      if available_roots.len() == reaching_entry_like.len() + splittable_roots.len() {
        continue;
      }

      // Duplicate asset into ALL reaching entry-like bundles (matching JS algorithm).
      // Each entry-like bundle must independently contain every sync-reachable asset
      // because they can be loaded in isolation.
      //
      // If the asset's file type differs from the bundle's type (e.g. CSS asset in JS bundle),
      // redirect it into a type-change sibling bundle keyed by (root, file_type).
      for &root in &reaching_entry_like {
        // Skip if already available from an ancestor of this root.
        if available_roots.contains(&root) {
          continue;
        }

        let bundle_id = &root_bundle_ids[&root];
        let target_bundle_id =
          self.resolve_type_change_target(asset_key, root, bundle_id, ideal)?;

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
        let canonical_bundle_id =
          self.resolve_type_change_target(asset_key, canonical, &parent_bundle_id, ideal)?;
        // Only record canonical placement here (asset may have been duplicated into multiple bundles).
        // `move_asset_to_bundle` would remove it from the other entry-like bundles, which we don't want.
        ideal.set_asset_bundle(asset_key, Some(canonical_bundle_id));
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
        .into_iter()
        .filter(|root| {
          root_bundle_ids
            .get(root)
            .and_then(|id| ideal.get_bundle(id))
            .is_none_or(|b| !b.ancestor_assets.contains(asset_key.0))
        })
        .collect();

      eligible.sort();
      eligible.dedup();

      if eligible.len() <= 1 {
        // JS: if reachableArray.length <= 1, no shared bundle is created.
        // - eligible == 0: available everywhere
        // - eligible == 1: the asset is unique to a single root after filtering; no sharing needed
        // Placement into a single root should already have happened in Phase 7.
        continue;
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
        let bundle = ideal
          .get_bundle_mut(&shared_bundle_id)
          .context("shared bundle missing")?;
        bundle.assets.insert(asset.0 as usize);
        ideal.move_asset_to_bundle(asset, &shared_bundle_id)?;

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
  #[instrument(level = "debug", skip_all)]
  fn internalize_async_bundles(
    &mut self,
    asset_graph: &AssetGraph,
    reachability: &Reachability,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Build reverse lookup: target asset id -> deps (priority, source asset id) that point to it.
    // This avoids repeated full scans of `asset_graph.get_dependencies()`.
    let mut deps_targeting: HashMap<String, Vec<(Priority, Option<String>)>> = HashMap::new();
    for dep in asset_graph.get_dependencies() {
      if dep.is_entry {
        continue;
      }
      let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dep.id) else {
        continue;
      };
      for neighbor in asset_graph.get_outgoing_neighbors(dep_node_id) {
        if let Some(asset) = asset_graph.get_asset(&neighbor) {
          deps_targeting
            .entry(asset.id.clone())
            .or_default()
            .push((dep.priority, dep.source_asset_id.clone()));
        }
      }
    }

    // Precompute bundle parent adjacency for fast ancestor walking (child -> parents).
    let mut bundle_parents: HashMap<IdealBundleId, Vec<IdealBundleId>> = HashMap::new();
    for (from, to, _ty) in &ideal.bundle_edges {
      bundle_parents.entry(*to).or_default().push(*from);
    }

    // Precompute reverse adjacency for sync edges in the *asset graph*:
    // target asset key -> source asset keys that sync-import it.
    let mut sync_incoming: Vec<Vec<AssetKey>> = vec![Vec::new(); self.assets.len()];
    for (target_id, deps) in &deps_targeting {
      let Some(target_key) = self.assets.key_for(target_id) else {
        continue;
      };
      let incoming = &mut sync_incoming[target_key.0 as usize];
      for (p, source_asset_id) in deps {
        if *p != Priority::Sync {
          continue;
        }
        let Some(source_asset_id) = source_asset_id else {
          continue;
        };
        let Some(source_key) = self.assets.key_for(source_asset_id) else {
          continue;
        };
        incoming.push(source_key);
      }
    }

    // Collect async bundle roots: non-entry bundle boundaries that became
    // boundaries due to a lazy/async dependency (not just type change).
    let async_roots: Vec<AssetKey> = self
      .bundle_boundaries
      .iter()
      .copied()
      .filter(|k| {
        if self.entry_roots.contains(k) {
          return false;
        }
        let asset_id = self.assets.id_for(*k);
        deps_targeting
          .get(asset_id)
          .is_some_and(|deps| deps.iter().any(|(p, _)| *p == Priority::Lazy))
      })
      .collect();

    // Precompute reverse sync reachability for all async roots using batch propagation.
    //
    // For each asset, track which async roots can sync-reach it (via reverse sync edges in the asset graph).
    let num_async = async_roots.len();
    let num_assets = self.assets.len();

    let async_root_to_bit: HashMap<AssetKey, usize> = async_roots
      .iter()
      .enumerate()
      .map(|(i, &k)| (k, i))
      .collect();

    // reverse_reach[asset_idx] = bitset of async-root bits that can reverse-sync-reach this asset.
    let mut reverse_reach: Vec<FixedBitSet> =
      vec![FixedBitSet::with_capacity(num_async); num_assets];

    for (bit, &root_key) in async_roots.iter().enumerate() {
      let idx = root_key.0 as usize;
      if idx < num_assets {
        reverse_reach[idx].insert(bit);
      }
    }

    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut in_queue: FixedBitSet = FixedBitSet::with_capacity(num_assets);

    for &root_key in &async_roots {
      let idx = root_key.0 as usize;
      if idx < num_assets {
        queue.push_back(idx);
        in_queue.insert(idx);
      }
    }

    while let Some(idx) = queue.pop_front() {
      in_queue.set(idx, false);

      // Clone to avoid borrowing `reverse_reach` immutably while mutating predecessors.
      let bits = reverse_reach[idx].clone();
      if bits.is_empty() {
        continue;
      }

      if idx >= sync_incoming.len() {
        continue;
      }

      for &predecessor in &sync_incoming[idx] {
        let pred_idx = predecessor.0 as usize;
        if pred_idx >= num_assets {
          continue;
        }

        let old_count = reverse_reach[pred_idx].count_ones(..);
        reverse_reach[pred_idx].union_with(&bits);
        let new_count = reverse_reach[pred_idx].count_ones(..);

        if new_count != old_count && !in_queue.contains(pred_idx) {
          queue.push_back(pred_idx);
          in_queue.insert(pred_idx);
        }
      }
    }

    // Transpose: reverse_reach_by_root[bit] = bitset of asset indices that can sync-reach async_roots[bit].
    let mut reverse_reach_by_root: Vec<FixedBitSet> =
      vec![FixedBitSet::with_capacity(num_assets); num_async];
    for (idx, reach_bits) in reverse_reach.iter().enumerate() {
      for bit in reach_bits.ones() {
        reverse_reach_by_root[bit].insert(idx);
      }
    }

    // Free the intermediate per-asset bitsets (~90MB at xlarge) to reduce memory pressure.
    drop(reverse_reach);

    let mut internalized: Vec<(IdealBundleId, Vec<IdealBundleId>)> = Vec::new();

    for &root_key in &async_roots {
      let root_id = self.assets.id_for(root_key);
      let bundle_id = IdealBundleId::from_asset_key(root_key);

      // Internalization guard.
      //
      // Only internalize async roots that are effectively "redundant" for *some* reason:
      // - A single source asset both lazy-imports and sync-imports the root (classic redundant async).
      // - OR an entry root sync-imports the root (the root is already in the entry-like bundle).
      //
      // Internalization is always eligible here.
      //
      // The JS bundler's `deleteBundle()` behavior simply removes the async bundle without
      // merging its assets into parents. Since we match that behavior, we don't need the
      // more conservative eligibility guard that would be required if we were copying
      // assets across bundle boundaries.
      let eligible_for_internalization = true;

      if !eligible_for_internalization {
        continue;
      }

      let Some(bundle) = ideal.get_bundle(&bundle_id) else {
        continue;
      };

      // Don't internalize isolated bundles.
      if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
      {
        continue;
      }

      // Find all parent bundles by looking at incoming dependencies in the *asset graph*.
      //
      // We can't rely solely on `ideal.bundle_edges` here because bundle edges are computed
      // before Phase 7 asset placement, and therefore may miss edges that originate from
      // non-root assets (e.g. an entry bundle asset that lazy-imports another bundle root).
      let mut parent_bundle_ids: Vec<IdealBundleId> = deps_targeting
        .get(root_id)
        .into_iter()
        .flatten()
        .filter_map(|(_p, source_asset_id)| {
          source_asset_id.as_ref().and_then(|sid| {
            self
              .assets
              .key_for(sid)
              .and_then(|k| ideal.asset_bundle(&k))
          })
        })
        .collect();
      parent_bundle_ids.sort();
      parent_bundle_ids.dedup();

      if parent_bundle_ids.is_empty() {
        continue;
      }

      // Check if the bundle root asset is available from ALL parents.
      // An asset is "available" from a parent if:
      // 1. It's directly in the parent bundle's assets, OR
      // 2. It's in the parent bundle's ancestor_assets, OR
      // 3. The parent can sync-reach the bundle root (via sync graph walk).
      let mut can_internalize = true;
      for parent_id in &parent_bundle_ids {
        let Some(parent_bundle) = ideal.get_bundle(parent_id) else {
          can_internalize = false;
          break;
        };

        let in_bundle = parent_bundle.assets.contains(root_key.0 as usize);
        let in_ancestors = parent_bundle.ancestor_assets.contains(root_key.0);

        let reachable_from_parent = if !in_bundle && !in_ancestors {
          // The JS algorithm considers an async bundle redundant if the async root is already
          // available when each parent loads. Importantly, this availability can come from
          // an *ancestor* bundle of the parent (e.g. the entry bundle), not necessarily from
          // the parent bundle itself.
          let root_bit = async_root_to_bit[&root_key];
          let reverse_sync_reachers = &reverse_reach_by_root[root_bit];

          self.is_asset_available_from_bundle_or_ancestors(
            reachability,
            ideal,
            &bundle_parents,
            reverse_sync_reachers,
            parent_id,
            root_key,
          )
        } else {
          true
        };

        if !reachable_from_parent {
          can_internalize = false;
          break;
        }
      }

      if can_internalize {
        internalized.push((bundle_id, parent_bundle_ids.clone()));
        self.decision(
          "internalization",
          super::types::DecisionKind::BundleInternalized {
            bundle_root: root_key,
          },
        );
      }
    }

    // Remove internalized bundles. Unlike the previous approach of copying assets
    // into parent bundles, we match the JS bundler's behavior: the bundle is simply
    // deleted. Parent bundles mark the root as "internalized" (handled during
    // NativeBundleGraph conversion in mod.rs), meaning the runtime won't try to
    // load the bundle separately.
    //
    // Also remove the root from `bundle_roots` so that Phase 9 (shared bundles) can
    // re-process the asset as a regular multi-root candidate — matching JS where
    // `deleteBundle` calls `bundleRoots.delete(bundleRoot)`.
    for (bundle_id, parents) in &internalized {
      let root_key = bundle_id.as_asset_key();

      // Match JS bundler behavior: once internalized, the async root is no longer treated as
      // its own bundle root, and should be re-processed by later phases as a regular asset.
      self.bundle_roots.remove(&root_key);

      ideal
        .internalized_bundles
        .insert(*bundle_id, parents.clone());

      // Clear the internalized bundle's assets and asset_to_bundle so Phase 9 can re-place the
      // root as a shared bundle candidate.
      if let Some(bundle) = ideal.get_bundle_mut(bundle_id) {
        bundle.assets.clear();
      }
      if let Some(entry) = ideal.asset_to_bundle.get_mut(root_key.0 as usize) {
        *entry = None;
      }
    }

    if !internalized.is_empty() {
      debug!(
        internalized = internalized.len(),
        "ideal graph: internalized async bundles"
      );
    }

    Ok(())
  }

  // ----------------------------
  // Helpers
  // ----------------------------

  /// Check if `from_id` can sync-reach `to_id` in the asset graph.
  /// Unlike the sync graph, this traversal crosses bundle boundaries.
  fn is_asset_available_from_bundle_or_ancestors(
    &self,
    reachability: &Reachability,
    ideal: &IdealGraph,
    bundle_parents: &HashMap<IdealBundleId, Vec<IdealBundleId>>,
    reverse_sync_reachers: &FixedBitSet,
    from_bundle_id: &IdealBundleId,
    asset_key: AssetKey,
  ) -> bool {
    let mut stack: Vec<IdealBundleId> = vec![*from_bundle_id];
    let mut visited: HashSet<IdealBundleId> = HashSet::new();

    // Primary fast path: check reachability from this bundle's root asset using Phase 5 reachability.
    //
    // If the async root asset is sync-reachable from a given bundle's root asset in the sync graph,
    // treat it as available.
    let asset_sync_node_idx = self.asset_to_sync_node.get(&asset_key).copied();

    while let Some(id) = stack.pop() {
      if !visited.insert(id) {
        continue;
      }

      if let Some(bundle) = ideal.get_bundle(&id)
        && (bundle.assets.contains(asset_key.0 as usize)
          || bundle.ancestor_assets.contains(asset_key.0))
      {
        return true;
      }

      // Check sync-reachability from this bundle's root asset using Phase 5 reachability.
      //
      // IMPORTANT: for bundle-root boundary assets (async roots and type-change boundaries),
      // we must NOT treat "sync-reachable" as "available" for internalization.
      // Otherwise, allowing sync traversal through boundaries would incorrectly internalize
      // async bundles whose *root* becomes reachable via another bundle's sync tree.
      if !self.bundle_boundaries.contains(&asset_key) {
        if let Some(asset_sync_node_idx) = asset_sync_node_idx {
          let from_root_key = id.as_asset_key();
          if let Some(&root_bit) = reachability.root_to_bit.get(&from_root_key)
            && reachability.reach_bits[asset_sync_node_idx.index()].contains(root_bit)
          {
            return true;
          }
        }
      }

      // Check sync-reachability from any asset in this bundle.
      if let Some(bundle) = ideal.get_bundle(&id)
        && bundle
          .assets
          .ones()
          .any(|idx| reverse_sync_reachers.contains(idx))
      {
        return true;
      }

      // Walk to ancestor bundles (those that have edges to `id`).
      if let Some(parents) = bundle_parents.get(&id) {
        stack.extend(parents.iter().cloned());
      }
    }

    false
  }

  fn asset_node_ids(&self, asset_graph: &AssetGraph) -> Vec<NodeId> {
    // We don't have direct access to internal indices, so we resolve from content-key map.
    asset_graph
      .get_assets()
      .filter_map(|a| asset_graph.get_node_id_by_content_key(&a.id).copied())
      .collect()
  }
}
