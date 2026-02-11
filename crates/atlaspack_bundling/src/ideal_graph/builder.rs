use std::collections::{HashMap, HashSet};

use fixedbitset::FixedBitSet;

#[derive(Debug, Default)]
struct AssetKeyInterner {
  by_id: HashMap<String, super::types::AssetKey>,
  ids: Vec<String>,
}

impl AssetKeyInterner {
  fn from_asset_graph(asset_graph: &AssetGraph) -> Self {
    let mut ids: Vec<String> = asset_graph.get_assets().map(|a| a.id.clone()).collect();
    ids.sort();
    ids.dedup();

    let mut by_id = HashMap::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
      let key = super::types::AssetKey(u32::try_from(i).expect("too many assets to key"));
      by_id.insert(id.clone(), key);
    }

    Self { by_id, ids }
  }

  fn key_for(&self, asset_id: &str) -> Option<super::types::AssetKey> {
    self.by_id.get(asset_id).copied()
  }

  fn id_for(&self, key: super::types::AssetKey) -> &str {
    &self.ids[key.0 as usize]
  }

  fn len(&self) -> usize {
    self.ids.len()
  }

  /// Convert a FixedBitSet back to a HashSet<String>.
  fn strings_from_bitset(&self, bs: &FixedBitSet) -> HashSet<String> {
    bs.ones().map(|i| self.ids[i].clone()).collect()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SyncNode {
  VirtualRoot,
  Asset(super::types::AssetKey),
}

use anyhow::Context;
use atlaspack_core::{
  asset_graph::{AssetGraph, NodeId},
  types::{BundleBehavior, Priority},
};
use petgraph::{
  Direction,
  algo::{dominators, kosaraju_scc},
  graph::NodeIndex,
  stable_graph::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences},
};
use tracing::{debug, instrument};

use super::types::{
  AssetKey, IdealBundle, IdealBundleId, IdealEdgeType, IdealGraph, IdealGraphBuildOptions,
  IdealGraphBuildStats,
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
  assets: AssetKeyInterner,
  bundle_boundaries: HashSet<super::types::AssetKey>,

  sync_graph: StableDiGraph<SyncNode, ()>,
  virtual_root: Option<NodeIndex>,
  asset_to_sync_node: HashMap<super::types::AssetKey, NodeIndex>,

  dominators: Option<dominators::Dominators<NodeIndex>>,

  // Entry roots.
  entry_roots: HashSet<super::types::AssetKey>,

  // Bundle roots are either entry assets or boundary assets.
  bundle_roots: HashSet<super::types::AssetKey>,

  // Asset key -> file type, populated during asset interning.
  asset_file_types: HashMap<super::types::AssetKey, atlaspack_core::types::FileType>,

  // Bundle roots that should be treated like entries (assets duplicated into them).
  // Includes entries, non-splittable, isolated, and stable-name bundle roots.
  entry_like_roots: HashSet<super::types::AssetKey>,
}

impl IdealGraphBuilder {
  fn decision(&mut self, phase: &'static str, kind: super::types::DecisionKind) {
    self.decisions.push(phase, kind);
  }

  pub fn new(options: IdealGraphBuildOptions) -> Self {
    Self {
      options,
      decisions: super::types::DecisionLog::default(),
      ..Self::default()
    }
  }

  /// Full pipeline entrypoint.
  #[instrument(level = "debug", skip_all)]
  pub fn build(
    mut self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<(IdealGraph, IdealGraphBuildStats)> {
    self.assets = AssetKeyInterner::from_asset_graph(asset_graph);
    for asset in asset_graph.get_assets() {
      if let Some(key) = self.assets.key_for(&asset.id) {
        self.asset_file_types.insert(key, asset.file_type.clone());
      }
    }
    debug!(
      interned_assets = self.assets.ids.len(),
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

    // Phase 2: build sync-only graph used for dominators.
    self.build_sync_graph(asset_graph, &entry_assets)?;

    // Phase 3: compute dominators.
    self.compute_dominators()?;

    // Phase 4: create bundle root shells (no non-root asset placement yet).
    let mut ideal = self.create_bundle_roots(asset_graph, &entry_assets)?;

    // Phase 5: bundle dependency graph (edge types).
    self.build_bundle_edges(asset_graph, &mut ideal)?;

    // Phase 6: derive reachability from dominator tree + sync graph.
    let reachability = self.compute_reachability_from_dominators()?;

    // Phase 7: place single-root assets into their dominating bundle.
    self.place_single_root_assets(&reachability, &mut ideal)?;

    // Phase 8a: availability propagation (now includes placed single-root assets).
    self.compute_availability(&mut ideal)?;

    // Phase 8b: extract shared bundles for multi-root assets.
    self.create_shared_bundles(&reachability, &mut ideal)?;

    // Phase 9: availability propagation again after shared bundle insertion.
    self.compute_availability(&mut ideal)?;

    // Phase 10: internalize async bundles whose root is already available from all parents.
    self.internalize_async_bundles(asset_graph, &mut ideal)?;

    // Always attach debug info. Decision payloads are compact and avoid String cloning.
    ideal.debug = Some(super::types::IdealGraphDebug {
      decisions: std::mem::take(&mut self.decisions),
      asset_ids: self.assets.ids.clone(),
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

          if dep_is_boundary || type_change || isolated {
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
    for asset_id in self.assets.ids.iter() {
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

          if self.bundle_boundaries.contains(&target_key) {
            continue;
          }

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
  // Phase 3: Dominators
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn compute_dominators(&mut self) -> anyhow::Result<()> {
    let root = self
      .virtual_root
      .context("sync graph virtual root not initialized")?;

    self.dominators = Some(dominators::simple_fast(&self.sync_graph, root));
    debug!("ideal graph: computed dominators");
    Ok(())
  }

  // ----------------------------
  // Phase 4: Placement
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

    let mut ideal = IdealGraph::default();

    for root_key in self.bundle_roots.clone() {
      let root_asset_id = self.assets.id_for(root_key).to_string();

      let asset = self
        .find_asset_by_id(asset_graph, &root_asset_id)
        .context("bundle root asset missing from graph")?;

      // Determine if this root should be treated like an entry (assets duplicated into it).
      let is_entry = self.entry_roots.contains(&root_key);
      let is_isolated = asset.bundle_behavior == Some(BundleBehavior::Isolated)
        || asset.bundle_behavior == Some(BundleBehavior::InlineIsolated);

      // Look up the dependency that targets this asset to check needs_stable_name.
      let dep_needs_stable_name = asset_graph
        .get_dependencies()
        .filter(|dep| !dep.is_entry)
        .any(|dep| {
          dep.needs_stable_name
            && asset_graph
              .get_node_id_by_content_key(&dep.id)
              .map(|dep_node| {
                asset_graph
                  .get_outgoing_neighbors(dep_node)
                  .iter()
                  .any(|n| {
                    asset_graph
                      .get_asset(n)
                      .map(|a| a.id == root_asset_id)
                      .unwrap_or(false)
                  })
              })
              .unwrap_or(false)
        });

      let needs_stable_name = is_entry || dep_needs_stable_name;

      if !asset.is_bundle_splittable || is_isolated || dep_needs_stable_name {
        self.entry_like_roots.insert(root_key);
      }

      let bundle_id = IdealBundleId(root_asset_id.clone());

      ideal.bundles.insert(
        bundle_id.clone(),
        IdealBundle {
          id: bundle_id.clone(),
          root_asset_id: Some(root_asset_id.clone()),
          assets: HashSet::from([root_asset_id.clone()]),
          bundle_type: asset.file_type.clone(),
          needs_stable_name,
          behavior: asset.bundle_behavior,
          ancestor_assets: HashSet::new(),
        },
      );

      ideal
        .asset_to_bundle
        .insert(root_asset_id.clone(), bundle_id.clone());

      self.decision(
        "placement",
        super::types::DecisionKind::BundleRootCreated {
          root_asset: root_key,
        },
      );
    }

    debug!(
      bundles = ideal.bundles.len(),
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

    for asset_node_id in self.asset_node_ids(asset_graph) {
      let Some(from_asset) = asset_graph.get_asset(&asset_node_id) else {
        continue;
      };

      let Some(from_bundle) = ideal.asset_to_bundle.get(&from_asset.id) else {
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

          let to_bundle = IdealBundleId(target_asset.id.clone());
          if *from_bundle == to_bundle {
            continue;
          }

          ideal
            .bundle_edges
            .push((from_bundle.clone(), to_bundle, edge_type));
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
  // Phase 6: Availability propagation
  // ----------------------------

  /// Computes `ancestor_assets` per bundle using the *intersection* rule described in the doc.
  ///
  /// Cycle handling:
  /// - Fast path: if the bundle graph is a DAG, we do a single topo-order propagation.
  /// - Slow path: if the bundle graph contains cycles, we condense strongly connected components (SCCs)
  ///   into a DAG and propagate at SCC granularity.
  ///
  /// The SCC path is intentionally isolated so it can be deleted easily if we decide the bundle graph
  /// must be acyclic (e.g. by construction) and we no longer want to support cycles.
  ///
  /// This is currently implemented with `HashSet<String>` for simplicity. We can swap this
  /// for a bitset representation once the algorithm is stable.
  #[instrument(level = "debug", skip_all)]
  fn compute_availability(&mut self, ideal: &mut IdealGraph) -> anyhow::Result<()> {
    // Build a petgraph representation for traversal.
    let mut g: StableDiGraph<IdealBundleId, IdealEdgeType> = StableDiGraph::new();
    let mut idx_by_bundle: HashMap<IdealBundleId, NodeIndex> = HashMap::new();

    for bundle_id in ideal.bundles.keys() {
      idx_by_bundle.insert(bundle_id.clone(), g.add_node(bundle_id.clone()));
    }

    for (from, to, ty) in ideal.bundle_edges.iter().cloned() {
      let Some(&from_idx) = idx_by_bundle.get(&from) else {
        continue;
      };
      let Some(&to_idx) = idx_by_bundle.get(&to) else {
        continue;
      };
      g.add_edge(from_idx, to_idx, ty);
    }

    // Default: root bundles start with empty availability.
    for bundle in ideal.bundles.values_mut() {
      bundle.ancestor_assets.clear();
    }

    // ----------------------------
    // Fast path: DAG topo propagation
    // ----------------------------
    if let Ok(order) = petgraph::algo::toposort(&g, None) {
      self.compute_availability_dag(&g, ideal, order)?;
      debug!(
        bundles = ideal.bundles.len(),
        "ideal graph: computed availability (dag)"
      );
      return Ok(());
    }

    // ----------------------------
    // Cycle-handling path: SCC condensation
    // ----------------------------
    debug!("ideal graph: bundle graph has cycles; computing availability via SCC condensation");
    self.compute_availability_scc(&g, ideal)?;
    debug!(
      bundles = ideal.bundles.len(),
      "ideal graph: computed availability (scc)"
    );
    Ok(())
  }

  /// Compute a FixedBitSet representing all assets available when a bundle loads
  /// (its own assets + ancestor_assets).
  fn bundle_available_bitset(&self, bundle: &IdealBundle) -> FixedBitSet {
    let capacity = self.assets.len();
    let mut bs = FixedBitSet::with_capacity(capacity);
    for id in &bundle.assets {
      if let Some(key) = self.assets.key_for(id) {
        bs.insert(key.0 as usize);
      }
    }
    for id in &bundle.ancestor_assets {
      if let Some(key) = self.assets.key_for(id) {
        bs.insert(key.0 as usize);
      }
    }
    bs
  }

  fn compute_availability_dag(
    &mut self,
    g: &StableDiGraph<IdealBundleId, IdealEdgeType>,
    ideal: &mut IdealGraph,
    order: Vec<NodeIndex>,
  ) -> anyhow::Result<()> {
    let capacity = self.assets.len();

    // Store computed availability per node as FixedBitSet.
    let mut node_availability: HashMap<NodeIndex, Option<FixedBitSet>> = HashMap::new();

    for parent_node in &order {
      let parent_id = g
        .node_weight(*parent_node)
        .cloned()
        .context("parent node missing")?;

      // What this parent makes available to its children (bitset).
      let parent_available = ideal
        .bundles
        .get(&parent_id)
        .map(|b| self.bundle_available_bitset(b))
        .unwrap_or_else(|| FixedBitSet::with_capacity(capacity));

      // Collect children grouped by edge type, maintaining stable order.
      let mut children: Vec<(NodeIndex, IdealEdgeType)> = Vec::new();
      for child_node in g.neighbors_directed(*parent_node, Direction::Outgoing) {
        if let Some(edge) = g.find_edge(*parent_node, child_node)
          && let Some(edge_type) = g.edge_weight(edge)
        {
          children.push((child_node, *edge_type));
        }
      }

      // Sort children deterministically for parallel ordering.
      children.sort_by(|(a, _), (b, _)| {
        let a_id = g.node_weight(*a).map(|id| id.0.as_str()).unwrap_or("");
        let b_id = g.node_weight(*b).map(|id| id.0.as_str()).unwrap_or("");
        a_id.cmp(b_id)
      });

      // Track parallel sibling availability (running union of earlier parallel siblings).
      let mut parallel_availability = FixedBitSet::with_capacity(capacity);

      for (child_node, edge_type) in children {
        let child_id = g
          .node_weight(child_node)
          .cloned()
          .context("child node missing")?;

        // Skip children with bundle behavior (isolated, etc).
        let child_is_isolated = ideal
          .bundles
          .get(&child_id)
          .map(|b| {
            b.behavior == Some(BundleBehavior::Isolated)
              || b.behavior == Some(BundleBehavior::InlineIsolated)
          })
          .unwrap_or(false);

        if child_is_isolated {
          node_availability.insert(child_node, Some(FixedBitSet::with_capacity(capacity)));
          continue;
        }

        // Compute what this parent offers to this specific child.
        let current_available = if edge_type == IdealEdgeType::Parallel {
          let mut combined = parent_available.clone();
          combined.union_with(&parallel_availability);
          combined
        } else {
          parent_available.clone()
        };

        // Intersect with any previously computed availability (from other parents).
        let merged = match node_availability.remove(&child_node).flatten() {
          None => current_available,
          Some(mut prev) => {
            prev.intersect_with(&current_available);
            prev
          }
        };
        node_availability.insert(child_node, Some(merged));

        // If this child is parallel, add its assets to parallel availability.
        if edge_type == IdealEdgeType::Parallel
          && let Some(child_bundle) = ideal.bundles.get(&child_id)
        {
          for id in &child_bundle.assets {
            if let Some(key) = self.assets.key_for(id) {
              parallel_availability.insert(key.0 as usize);
            }
          }
        }
      }
    }

    // Assign computed availability to bundles.
    for node in &order {
      let bundle_id = g
        .node_weight(*node)
        .cloned()
        .context("bundle node missing")?;

      let availability = node_availability
        .get(node)
        .and_then(|v| v.as_ref())
        .map(|bs| self.assets.strings_from_bitset(bs))
        .unwrap_or_default();

      let bundle = ideal
        .bundles
        .get_mut(&bundle_id)
        .context("bundle missing")?;

      // Isolated bundles do not inherit availability.
      if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
      {
        bundle.ancestor_assets = HashSet::new();
      } else {
        bundle.ancestor_assets = availability;
      }

      self.decision(
        "availability",
        super::types::DecisionKind::AvailabilityComputed {
          bundle_root: self
            .assets
            .key_for(&bundle_id.0)
            .context("bundle root not interned")?,
          ancestor_assets_len: bundle.ancestor_assets.len(),
        },
      );
    }

    Ok(())
  }

  fn compute_availability_scc(
    &mut self,
    g: &StableDiGraph<IdealBundleId, IdealEdgeType>,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // `kosaraju_scc` returns SCCs as lists of nodes.
    let sccs: Vec<Vec<NodeIndex>> = kosaraju_scc(g);

    // Map node -> scc index.
    let mut scc_of: HashMap<NodeIndex, usize> = HashMap::new();
    for (i, scc) in sccs.iter().enumerate() {
      for &n in scc {
        scc_of.insert(n, i);
      }
    }

    // Build SCC DAG: nodes are SCC indices.
    let mut scc_graph: StableDiGraph<usize, ()> = StableDiGraph::new();
    let scc_nodes: Vec<NodeIndex> = (0..sccs.len()).map(|i| scc_graph.add_node(i)).collect();

    for e in g.edge_references() {
      let a = e.source();
      let b = e.target();
      let sa = scc_of[&a];
      let sb = scc_of[&b];
      if sa != sb {
        scc_graph.add_edge(scc_nodes[sa], scc_nodes[sb], ());
      }
    }

    // Condensation graph is a DAG.
    let scc_order = match petgraph::algo::toposort(&scc_graph, None) {
      Ok(o) => o,
      Err(_) => {
        // Condensation graph should always be acyclic.
        anyhow::bail!("SCC condensation graph unexpectedly cyclic");
      }
    };

    let capacity = self.assets.len();

    // Compute an "available set" per SCC as FixedBitSet.
    let mut available_for_scc: Vec<FixedBitSet> =
      vec![FixedBitSet::with_capacity(capacity); sccs.len()];

    for scc_node in scc_order {
      let scc_idx = *scc_graph
        .node_weight(scc_node)
        .context("SCC node missing")?;

      // Parents are other SCCs.
      let parents: Vec<usize> = scc_graph
        .neighbors_directed(scc_node, Direction::Incoming)
        .filter_map(|p| scc_graph.node_weight(p).copied())
        .collect();

      let mut availability: Option<FixedBitSet> = None;
      for p in parents {
        let parent_bits = available_for_scc[p].clone();
        availability = Some(match availability {
          None => parent_bits,
          Some(mut prev) => {
            prev.intersect_with(&parent_bits);
            prev
          }
        });
      }
      let availability_bs = availability.unwrap_or_else(|| FixedBitSet::with_capacity(capacity));
      let availability_strings = self.assets.strings_from_bitset(&availability_bs);

      // Assign availability to each bundle in the SCC.
      for &bundle_node in &sccs[scc_idx] {
        let bundle_id = g
          .node_weight(bundle_node)
          .cloned()
          .context("bundle node missing")?;

        let bundle = ideal
          .bundles
          .get_mut(&bundle_id)
          .context("bundle missing")?;

        if bundle.behavior == Some(BundleBehavior::Isolated)
          || bundle.behavior == Some(BundleBehavior::InlineIsolated)
        {
          bundle.ancestor_assets = HashSet::new();
        } else {
          bundle.ancestor_assets = availability_strings.clone();
        }

        self.decision(
          "availability",
          super::types::DecisionKind::AvailabilityComputed {
            bundle_root: self
              .assets
              .key_for(&bundle_id.0)
              .context("bundle root not interned")?,
            ancestor_assets_len: bundle.ancestor_assets.len(),
          },
        );
      }

      // After we assign ancestor assets for SCC, compute what becomes available to children SCCs.
      // We conservatively use the union of all assets available from bundles in this SCC.
      let mut scc_available = FixedBitSet::with_capacity(capacity);
      for &bundle_node in &sccs[scc_idx] {
        let bundle_id = g
          .node_weight(bundle_node)
          .cloned()
          .context("bundle node missing")?;
        if let Some(b) = ideal.bundles.get(&bundle_id) {
          scc_available.union_with(&self.bundle_available_bitset(b));
        }
      }

      available_for_scc[scc_idx] = scc_available;
    }

    Ok(())
  }

  // ----------------------------
  // Phase 7: Dominator-based reachability
  // ----------------------------

  /// Derive reachability from the dominator tree structure.
  ///
  /// For each non-root asset:
  /// - Walk up the dominator tree. If we hit a bundle root before reaching virtual_root,
  ///   that asset is dominated by (and reachable from) exactly that one root.
  /// - If idom chain reaches virtual_root, the asset is reachable from multiple roots.
  ///   We find those roots via a reverse walk on the sync graph.
  ///
  /// Returns a map of asset -> set of bundle roots that can reach it.
  #[instrument(level = "debug", skip_all)]
  fn compute_reachability_from_dominators(
    &mut self,
  ) -> anyhow::Result<HashMap<AssetKey, HashSet<AssetKey>>> {
    let doms = self
      .dominators
      .as_ref()
      .context("dominators not computed")?;

    let virtual_root = self
      .virtual_root
      .context("sync graph virtual root not initialized")?;

    // Build dominator tree child lists for efficient subtree traversal.
    // parent_node -> [child_nodes]
    let mut dom_children: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for (&asset_key, &node_idx) in &self.asset_to_sync_node {
      // Skip bundle roots -- they are placed in Phase 4 already.
      if self.bundle_roots.contains(&asset_key) {
        continue;
      }
      if let Some(idom) = doms.immediate_dominator(node_idx) {
        dom_children.entry(idom).or_default().push(node_idx);
      }
    }

    // For each bundle root, collect the assets in its dominator subtree.
    // These are assets dominated by that root and reachable only from it.
    let mut asset_roots: HashMap<AssetKey, HashSet<AssetKey>> = HashMap::new();

    let roots: Vec<AssetKey> = self.bundle_roots.iter().copied().collect();
    for root_key in &roots {
      let Some(&root_idx) = self.asset_to_sync_node.get(root_key) else {
        continue;
      };

      // DFS through dominator subtree of this root.
      let mut stack = vec![root_idx];
      while let Some(node) = stack.pop() {
        if let Some(children) = dom_children.get(&node) {
          for &child in children {
            if let Some(SyncNode::Asset(child_key)) = self.sync_graph.node_weight(child).copied() {
              asset_roots.entry(child_key).or_default().insert(*root_key);
            }
            stack.push(child);
          }
        }
      }
    }

    for &root_key in &roots {
      self.decision(
        "reachability",
        super::types::DecisionKind::ReachabilityComputed {
          bundle_root: root_key,
          reachable_assets_len: asset_roots
            .values()
            .filter(|r| r.contains(&root_key))
            .count(),
        },
      );
    }

    // For assets whose idom is virtual_root (multi-root), the dominator subtree
    // traversal above won't find them under any single root. Their idom children
    // sit directly under virtual_root. We need to find which roots can reach them
    // via a reverse walk on the sync graph.
    if let Some(vr_children) = dom_children.get(&virtual_root) {
      // Collect all multi-root assets: virtual_root's subtree in the dominator tree.
      let mut multi_root_assets: Vec<AssetKey> = Vec::new();
      let mut stack: Vec<NodeIndex> = vr_children.clone();
      while let Some(node) = stack.pop() {
        if let Some(SyncNode::Asset(key)) = self.sync_graph.node_weight(node).copied()
          && !self.bundle_roots.contains(&key)
        {
          multi_root_assets.push(key);
        }
        if let Some(children) = dom_children.get(&node) {
          stack.extend(children);
        }
      }

      // For each multi-root asset, reverse-walk the sync graph to find which bundle
      // roots can reach it.
      for asset_key in multi_root_assets {
        let Some(&asset_idx) = self.asset_to_sync_node.get(&asset_key) else {
          continue;
        };

        let mut reaching_roots: HashSet<AssetKey> = HashSet::new();
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut rev_stack: Vec<NodeIndex> = vec![asset_idx];

        while let Some(n) = rev_stack.pop() {
          if !visited.insert(n) {
            continue;
          }
          if let Some(SyncNode::Asset(k)) = self.sync_graph.node_weight(n).copied()
            && self.bundle_roots.contains(&k)
          {
            reaching_roots.insert(k);
            continue; // Don't walk past bundle roots.
          }
          // Walk predecessors (incoming edges in sync graph).
          for pred in self.sync_graph.neighbors_directed(n, Direction::Incoming) {
            // Skip virtual root.
            if pred == virtual_root {
              continue;
            }
            rev_stack.push(pred);
          }
        }

        asset_roots.insert(asset_key, reaching_roots);
      }
    }

    debug!(
      assets_with_reachability = asset_roots.len(),
      "ideal graph: computed dominator-based reachability"
    );
    Ok(asset_roots)
  }

  // ----------------------------
  // Phase 7: Single-root asset placement
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
    reachability: &HashMap<AssetKey, HashSet<AssetKey>>,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    for (&asset, roots) in reachability {
      // Bundle roots are already placed.
      if self.bundle_roots.contains(&asset) {
        continue;
      }

      // Entry-like roots: entries, non-splittable, isolated, needs-stable-name.
      // These get assets duplicated into them (same as entries in JS algorithm).
      let reaching_entry_like: Vec<AssetKey> = roots
        .iter()
        .copied()
        .filter(|r| self.entry_like_roots.contains(r))
        .collect();

      let splittable_roots: Vec<AssetKey> = roots
        .iter()
        .copied()
        .filter(|r| !self.entry_like_roots.contains(r))
        .collect();

      let asset_id_str = self.assets.id_for(asset).to_string();

      // Duplicate asset into ALL reaching entry-like bundles (matching JS algorithm).
      // Each entry-like bundle must independently contain every sync-reachable asset
      // because they can be loaded in isolation.
      for &root in &reaching_entry_like {
        let bundle_id = IdealBundleId(self.assets.id_for(root).to_string());
        let bundle = ideal
          .bundles
          .get_mut(&bundle_id)
          .context("entry-like bundle missing for duplication")?;
        bundle.assets.insert(asset_id_str.clone());

        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::SingleEligibleRoot,
          },
        );
      }

      // Track canonical bundle assignment (smallest entry-like for determinism).
      if !reaching_entry_like.is_empty() {
        let canonical = reaching_entry_like.iter().copied().min().unwrap();
        let bundle_id = IdealBundleId(self.assets.id_for(canonical).to_string());
        ideal
          .asset_to_bundle
          .insert(asset_id_str.clone(), bundle_id);
      }

      // If placed in entry-like bundles and no splittable roots remain (or multiple
      // that need shared extraction), defer to Phase 8.
      if !reaching_entry_like.is_empty() && splittable_roots.len() != 1 {
        continue;
      }

      if splittable_roots.len() > 1 {
        // Multi-root asset with no entry-like roots -> deferred to Phase 8.
        continue;
      } else if splittable_roots.len() == 1 {
        // Single splittable root -> place directly.
        let root = splittable_roots[0];
        let bundle_id = IdealBundleId(self.assets.id_for(root).to_string());
        let bundle = ideal
          .bundles
          .get_mut(&bundle_id)
          .context("bundle missing for single-root placement")?;
        bundle.assets.insert(asset_id_str.clone());
        ideal.asset_to_bundle.insert(asset_id_str, bundle_id);

        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::DominatorSubtree,
          },
        );
      }
    }

    debug!(
      placed = ideal.asset_to_bundle.len(),
      "ideal graph: placed single-root assets"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 8: Shared bundle creation
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
    reachability: &HashMap<AssetKey, HashSet<AssetKey>>,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    let mut eligible_roots_by_asset: HashMap<AssetKey, Vec<AssetKey>> = HashMap::new();

    for (&asset, roots) in reachability {
      if self.bundle_roots.contains(&asset) {
        continue;
      }

      // Only process multi-root assets (those skipped in Phase 7).
      let splittable_roots: Vec<AssetKey> = roots
        .iter()
        .copied()
        .filter(|r| !self.entry_like_roots.contains(r))
        .collect();

      if splittable_roots.len() <= 1 {
        continue; // Already placed in Phase 7.
      }

      let asset_id = self.assets.id_for(asset);

      // Filter by availability: only keep roots where asset is NOT already available.
      // An asset is considered available from root R if:
      // 1. It's in R's ancestor_assets (already in an ancestor bundle), OR
      // 2. Another root in the reachable set is an ancestor of R (the asset will
      //    be placed in that ancestor's bundle, making it available via bundle reuse).
      let mut eligible: Vec<AssetKey> = Vec::new();
      for &root in &splittable_roots {
        let root_id = self.assets.id_for(root);
        let bundle_id = IdealBundleId(root_id.to_string());
        let Some(bundle) = ideal.bundles.get(&bundle_id) else {
          continue;
        };

        // Check 1: standard availability.
        if bundle.ancestor_assets.contains(asset_id) {
          continue;
        }

        // Check 2: bundle reuse / subgraph detection.
        // If there's a bundle edge path from this root to another reachable root
        // that dominates the asset, the asset will be in that other root's bundle.
        // This root can load it via the edge rather than needing a shared bundle.
        let available_via_reuse = splittable_roots.iter().any(|&other_root| {
          if other_root == root {
            return false;
          }
          let other_bundle_id = IdealBundleId(self.assets.id_for(other_root).to_string());
          // Check if there's any bundle edge from this root to the other root.
          ideal
            .bundle_edges
            .iter()
            .any(|(from, to, _)| *from == bundle_id && *to == other_bundle_id)
        });

        if available_via_reuse {
          continue;
        }

        // Check 3: parallel sibling availability.
        // If this root has a parallel sibling that also reaches the asset and
        // comes before it in deterministic ordering, the asset will be placed
        // in the earlier sibling's bundle and available via parallel loading.
        let available_via_parallel = splittable_roots.iter().any(|&other_root| {
          if other_root >= root {
            // Only earlier siblings (deterministic order) provide availability.
            return false;
          }
          let other_bundle_id = IdealBundleId(self.assets.id_for(other_root).to_string());
          // Check if both roots share a common parent via parallel edges.
          ideal.bundle_edges.iter().any(|(parent, to, ty)| {
            *to == other_bundle_id
              && *ty == IdealEdgeType::Parallel
              && ideal.bundle_edges.iter().any(|(p2, t2, ty2)| {
                p2 == parent && *t2 == bundle_id && *ty2 == IdealEdgeType::Parallel
              })
          })
        });

        if available_via_parallel {
          continue;
        }

        eligible.push(root);
      }

      if eligible.len() > 1 {
        eligible.sort();
        eligible.dedup();
        eligible_roots_by_asset.insert(asset, eligible);
      } else if eligible.len() == 1 {
        // Availability reduced to single root -> place directly.
        let root = eligible[0];
        let bundle_id = IdealBundleId(self.assets.id_for(root).to_string());
        let bundle = ideal
          .bundles
          .get_mut(&bundle_id)
          .context("bundle missing for single-eligible placement")?;
        let asset_id_str = self.assets.id_for(asset).to_string();
        bundle.assets.insert(asset_id_str.clone());
        ideal.asset_to_bundle.insert(asset_id_str, bundle_id);

        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::SingleEligibleRoot,
          },
        );
      }
      // else: available from all roots -> no placement needed.
    }

    if eligible_roots_by_asset.is_empty() {
      debug!("ideal graph: no shared bundle candidates");
      return Ok(());
    }

    // Group assets by eligible root set (so we create fewer shared bundles).
    let mut assets_by_root_set: HashMap<Vec<AssetKey>, Vec<AssetKey>> = HashMap::new();
    for (asset, roots) in eligible_roots_by_asset {
      assets_by_root_set.entry(roots).or_default().push(asset);
    }

    for (mut roots, assets) in assets_by_root_set {
      roots.sort();
      roots.dedup();

      let shared_name = format!(
        "@@shared:{}",
        roots
          .iter()
          .map(|r| self.assets.id_for(*r))
          .collect::<Vec<_>>()
          .join(",")
      );

      let shared_key = {
        let next = super::types::AssetKey(u32::try_from(self.assets.ids.len()).unwrap());
        self.assets.by_id.insert(shared_name.clone(), next);
        self.assets.ids.push(shared_name.clone());
        next
      };

      let shared_bundle_id = super::types::IdealBundleId(shared_name.clone());

      // Derive bundle type from the first asset in the group.
      let bundle_type = assets
        .first()
        .and_then(|a| self.asset_file_types.get(a).cloned())
        .unwrap_or(atlaspack_core::types::FileType::Js);

      ideal.create_bundle(super::types::IdealBundle {
        id: shared_bundle_id.clone(),
        root_asset_id: None,
        assets: HashSet::new(),
        bundle_type,
        needs_stable_name: false,
        behavior: None,
        ancestor_assets: HashSet::new(),
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
        let asset_id = self.assets.id_for(asset).to_string();

        let bundle = ideal
          .bundles
          .get_mut(&shared_bundle_id)
          .context("shared bundle missing")?;
        bundle.assets.insert(asset_id.clone());
        ideal
          .asset_to_bundle
          .insert(asset_id, shared_bundle_id.clone());

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
        let from_id = IdealBundleId(self.assets.id_for(root).to_string());
        ideal.add_bundle_edge(from_id, shared_bundle_id.clone(), IdealEdgeType::Sync);
      }
    }

    debug!(
      bundles = ideal.bundles.len(),
      "ideal graph: shared bundles created"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 10: Internalization
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
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Collect async bundle roots: non-entry bundle boundaries that became
    // boundaries due to a lazy/async dependency (not just type change).
    // We check the asset graph's incoming dependencies directly.
    let async_roots: Vec<AssetKey> = self
      .bundle_boundaries
      .iter()
      .copied()
      .filter(|k| {
        if self.entry_roots.contains(k) {
          return false;
        }
        let asset_id = self.assets.id_for(*k);
        // Check if any dependency targeting this asset is lazy (async).
        asset_graph.get_dependencies().any(|dep| {
          dep.priority == Priority::Lazy
            && asset_graph
              .get_node_id_by_content_key(&dep.id)
              .map(|dep_node| {
                asset_graph
                  .get_outgoing_neighbors(dep_node)
                  .iter()
                  .any(|n| {
                    asset_graph
                      .get_asset(n)
                      .map(|a| a.id == asset_id)
                      .unwrap_or(false)
                  })
              })
              .unwrap_or(false)
        })
      })
      .collect();

    let mut internalized: Vec<IdealBundleId> = Vec::new();

    for root_key in async_roots {
      let root_id = self.assets.id_for(root_key).to_string();
      let bundle_id = IdealBundleId(root_id.clone());

      let Some(bundle) = ideal.bundles.get(&bundle_id) else {
        continue;
      };

      // Don't internalize isolated bundles.
      if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
      {
        continue;
      }

      // Find all parent bundles (bundles with an edge TO this bundle).
      let parent_bundle_ids: Vec<IdealBundleId> = ideal
        .bundle_edges
        .iter()
        .filter(|(_, to, _)| *to == bundle_id)
        .map(|(from, _, _)| from.clone())
        .collect();

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
        let Some(parent_bundle) = ideal.bundles.get(parent_id) else {
          can_internalize = false;
          break;
        };

        let in_bundle = parent_bundle.assets.contains(&root_id);
        let in_ancestors = parent_bundle.ancestor_assets.contains(&root_id);

        let reachable_from_parent = if !in_bundle && !in_ancestors {
          // Check if the parent bundle root can sync-reach this async root
          // via the asset graph (the sync graph excludes boundary edges).
          self.is_sync_reachable_in_asset_graph(asset_graph, &parent_id.0, &root_id)
        } else {
          true
        };

        if !reachable_from_parent {
          can_internalize = false;
          break;
        }
      }

      if can_internalize {
        internalized.push(bundle_id);
        self.decision(
          "internalization",
          super::types::DecisionKind::BundleInternalized {
            bundle_root: root_key,
          },
        );
      }
    }

    // Remove internalized bundles and place their assets into parent bundles.
    for bundle_id in &internalized {
      // Collect the bundle's assets before removing it.
      let bundle_assets: Vec<String> = ideal
        .bundles
        .get(bundle_id)
        .map(|b| b.assets.iter().cloned().collect())
        .unwrap_or_default();

      // Find parent bundles (those with edges TO this bundle).
      let parents: Vec<IdealBundleId> = ideal
        .bundle_edges
        .iter()
        .filter(|(_, to, _)| to == bundle_id)
        .map(|(from, _, _)| from.clone())
        .collect();

      // Add the internalized bundle's assets to each parent bundle.
      for parent_id in &parents {
        if let Some(parent_bundle) = ideal.bundles.get_mut(parent_id) {
          for asset_id in &bundle_assets {
            parent_bundle.assets.insert(asset_id.clone());
          }
        }
      }

      ideal.remove_bundle(bundle_id);
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
  fn is_sync_reachable_in_asset_graph(
    &self,
    asset_graph: &AssetGraph,
    from_id: &str,
    to_id: &str,
  ) -> bool {
    let Some(from_node) = asset_graph.get_node_id_by_content_key(from_id).copied() else {
      return false;
    };

    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack = vec![from_node];

    while let Some(node) = stack.pop() {
      if !visited.insert(node) {
        continue;
      }

      // Traverse asset -> dep -> asset edges, only through sync deps.
      for dep_node in asset_graph.get_outgoing_neighbors(&node) {
        let Some(dep) = asset_graph.get_dependency(&dep_node) else {
          continue;
        };
        if dep.priority != Priority::Sync {
          continue;
        }
        for target_node in asset_graph.get_outgoing_neighbors(&dep_node) {
          let Some(target) = asset_graph.get_asset(&target_node) else {
            continue;
          };
          if target.id == to_id {
            return true;
          }
          stack.push(target_node);
        }
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

  fn find_asset_by_id<'a>(
    &self,
    asset_graph: &'a AssetGraph,
    asset_id: &str,
  ) -> Option<&'a atlaspack_core::types::Asset> {
    asset_graph.get_assets().find(|a| a.id == asset_id)
  }
}
