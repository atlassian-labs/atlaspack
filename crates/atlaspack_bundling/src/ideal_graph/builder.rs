use fixedbitset::FixedBitSet;
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
  types::{BundleBehavior, Priority},
};
use petgraph::{
  Direction,
  algo::kosaraju_scc,
  graph::NodeIndex,
  stable_graph::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
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
  assets: super::types::AssetInterner,
  bundle_boundaries: HashSet<super::types::AssetKey>,

  sync_graph: StableDiGraph<SyncNode, ()>,
  virtual_root: Option<NodeIndex>,
  asset_to_sync_node: HashMap<super::types::AssetKey, NodeIndex>,

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

    // Phase 6: place single-root assets into their dominating bundle.
    self.place_single_root_assets(&reachability, &mut ideal)?;

    // Phase 7: availability propagation (now includes placed single-root assets).
    self.compute_availability(&mut ideal)?;

    // Phase 8: extract shared bundles for multi-root assets.
    self.create_shared_bundles(&reachability, &mut ideal)?;

    // Phase 9: availability propagation again after shared bundle insertion.
    self.compute_availability(&mut ideal)?;

    // Phase 10: internalize async bundles whose root is already available from all parents.
    self.internalize_async_bundles(asset_graph, &reachability, &mut ideal)?;

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
        ancestor_assets: FixedBitSet::with_capacity(self.assets.len()),
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

    for asset_node_id in self.asset_node_ids(asset_graph) {
      let Some(from_asset) = asset_graph.get_asset(&asset_node_id) else {
        continue;
      };

      let Some(from_key) = self.assets.key_for(&from_asset.id) else {
        continue;
      };

      let Some(from_bundle) = ideal.asset_bundle(&from_key) else {
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
  /// This is implemented with a bitset representation keyed by `AssetKey`.
  #[instrument(level = "debug", skip_all)]
  fn compute_availability(&mut self, ideal: &mut IdealGraph) -> anyhow::Result<()> {
    // Default: root bundles start with empty availability.
    for slot in ideal.bundles.iter_mut() {
      if let Some(bundle) = slot.as_mut() {
        bundle.ancestor_assets.clear();
      }
    }

    // ----------------------------
    // Fast path: DAG topo propagation (custom Kahn topo sort)
    // ----------------------------
    //
    // Building a full petgraph and running toposort is expensive at large bundle counts,
    // especially when the number of edges is small. We compute topo order directly from
    // `ideal.bundle_edges` and only fall back to SCC condensation when a cycle is detected.
    {
      let bundle_ids: Vec<IdealBundleId> = ideal.bundles_iter().map(|(id, _)| *id).collect();
      let mut indegree: HashMap<IdealBundleId, usize> = HashMap::new();
      for id in &bundle_ids {
        indegree.insert(*id, 0);
      }

      let mut outgoing: HashMap<IdealBundleId, Vec<(IdealBundleId, IdealEdgeType)>> =
        HashMap::new();
      for (from, to, ty) in &ideal.bundle_edges {
        outgoing.entry(*from).or_default().push((*to, *ty));
        if let Some(d) = indegree.get_mut(to) {
          *d += 1;
        }
      }

      // Deterministic ordering: collect all zero-indegree nodes and sort by id.
      let mut ready: Vec<IdealBundleId> = indegree
        .iter()
        .filter_map(|(id, d)| if *d == 0 { Some(*id) } else { None })
        .collect();
      ready.sort_by(|a, b| a.0.cmp(&b.0));

      let mut order: Vec<IdealBundleId> = Vec::with_capacity(indegree.len());
      while let Some(n) = ready.pop() {
        if let Some(children) = outgoing.get(&n) {
          for (to, _) in children {
            let d = indegree
              .get_mut(to)
              .expect("bundle missing from indegree map");
            *d -= 1;
            if *d == 0 {
              ready.push(*to);
            }
          }
          // Maintain deterministic ordering for newly-ready nodes.
          ready.sort_by(|a, b| b.0.cmp(&a.0));
        }
        order.push(n);
      }

      if order.len() == indegree.len() {
        self.compute_availability_dag_fast(ideal, &outgoing, &order)?;
        debug!(
          bundles = ideal.bundle_count(),
          "ideal graph: computed availability (dag)"
        );
        return Ok(());
      }
    }

    // ----------------------------
    // Cycle-handling path: SCC condensation (petgraph)
    // ----------------------------
    let mut g: StableDiGraph<IdealBundleId, IdealEdgeType> = StableDiGraph::new();
    let mut idx_by_bundle: HashMap<IdealBundleId, NodeIndex> = HashMap::new();

    for (bundle_id, _bundle) in ideal.bundles_iter() {
      idx_by_bundle.insert(*bundle_id, g.add_node(*bundle_id));
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

    debug!("ideal graph: bundle graph has cycles; computing availability via SCC condensation");
    self.compute_availability_scc(&g, ideal)?;
    debug!(
      bundles = ideal.bundle_count(),
      "ideal graph: computed availability (scc)"
    );
    Ok(())
  }

  /// Compute a FixedBitSet representing all assets available when a bundle loads
  /// (its own assets + ancestor_assets).
  fn bundle_available_bitset(&self, bundle: &IdealBundle) -> FixedBitSet {
    bundle.all_assets_available_from_here()
  }

  fn compute_availability_dag_fast(
    &mut self,
    ideal: &mut IdealGraph,
    outgoing: &HashMap<IdealBundleId, Vec<(IdealBundleId, IdealEdgeType)>>,
    order: &[IdealBundleId],
  ) -> anyhow::Result<()> {
    let capacity = self.assets.len();

    // Computed ancestor-availability per bundle as a FixedBitSet.
    // Indexed by `IdealBundleId.0 as usize`.
    let mut availability: Vec<Option<FixedBitSet>> = Vec::new();

    // Bundle assets as bitsets.
    // Indexed by `IdealBundleId.0 as usize`.
    let mut bundle_assets_bs: Vec<Option<FixedBitSet>> = Vec::new();

    for (id, bundle) in ideal.bundles_iter() {
      let idx = id.0 as usize;
      if bundle_assets_bs.len() <= idx {
        bundle_assets_bs.resize_with(idx + 1, || None);
      }
      bundle_assets_bs[idx] = Some(bundle.assets.clone());
    }

    for parent_id in order {
      let parent_is_isolated = ideal
        .get_bundle(parent_id)
        .map(|b| {
          b.behavior == Some(BundleBehavior::Isolated)
            || b.behavior == Some(BundleBehavior::InlineIsolated)
        })
        .unwrap_or(false);

      let parent_ancestor_bs = availability
        .get(parent_id.0 as usize)
        .and_then(|o| o.as_ref());

      // What this parent makes available to its children.
      let mut parent_available = bundle_assets_bs
        .get(parent_id.0 as usize)
        .and_then(|o| o.clone())
        .unwrap_or_else(|| FixedBitSet::with_capacity(capacity));
      if !parent_is_isolated && let Some(bs) = parent_ancestor_bs {
        parent_available.union_with(bs);
      }

      // Collect children and sort deterministically for parallel ordering.
      let mut children: Vec<(IdealBundleId, IdealEdgeType)> =
        outgoing.get(parent_id).cloned().unwrap_or_default();
      children.sort_by(|(a, _), (b, _)| a.0.cmp(&b.0));

      let mut parallel_availability = FixedBitSet::with_capacity(capacity);

      for (child_id, edge_type) in children {
        let child_is_isolated = ideal
          .get_bundle(&child_id)
          .map(|b| {
            b.behavior == Some(BundleBehavior::Isolated)
              || b.behavior == Some(BundleBehavior::InlineIsolated)
          })
          .unwrap_or(false);

        if child_is_isolated {
          let child_idx = child_id.0 as usize;
          if availability.len() <= child_idx {
            availability.resize_with(child_idx + 1, || None);
          }
          availability[child_idx] = Some(FixedBitSet::with_capacity(capacity));
          continue;
        }

        let current_available = if edge_type == IdealEdgeType::Parallel {
          let mut combined = parent_available.clone();
          combined.union_with(&parallel_availability);
          combined
        } else {
          parent_available.clone()
        };

        let child_idx = child_id.0 as usize;
        if availability.len() <= child_idx {
          availability.resize_with(child_idx + 1, || None);
        }

        match &mut availability[child_idx] {
          Some(prev) => {
            prev.intersect_with(&current_available);
          }
          None => {
            availability[child_idx] = Some(current_available);
          }
        }

        if edge_type == IdealEdgeType::Parallel {
          let child_assets = bundle_assets_bs
            .get(child_id.0 as usize)
            .and_then(|o| o.clone())
            .unwrap_or_else(|| FixedBitSet::with_capacity(capacity));
          parallel_availability.union_with(&child_assets);
        }
      }
    }

    // Final pass: assign `ancestor_assets` once per bundle.
    for bundle_id in order {
      let bundle = ideal.get_bundle_mut(bundle_id).context("bundle missing")?;

      if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
      {
        bundle.ancestor_assets.clear();
      } else {
        bundle.ancestor_assets = availability
          .get(bundle_id.0 as usize)
          .and_then(|o| o.clone())
          .unwrap_or_default();
      }

      self.decision(
        "availability",
        super::types::DecisionKind::AvailabilityComputed {
          bundle_root: bundle_id.as_asset_key(),
          ancestor_assets_len: bundle.ancestor_assets.count_ones(..),
        },
      );
    }

    Ok(())
  }

  #[allow(dead_code)]
  fn compute_availability_dag(
    &mut self,
    g: &StableDiGraph<IdealBundleId, IdealEdgeType>,
    ideal: &mut IdealGraph,
    order: Vec<NodeIndex>,
  ) -> anyhow::Result<()> {
    let capacity = self.assets.len();

    // Bundle assets as bitsets, cached lazily.
    let mut bundle_assets_bs: HashMap<IdealBundleId, FixedBitSet> = HashMap::new();
    let mut get_bundle_assets_bs = |bundle_id: &IdealBundleId| -> FixedBitSet {
      if let Some(bs) = bundle_assets_bs.get(bundle_id) {
        return bs.clone();
      }

      let bs = ideal
        .get_bundle(bundle_id)
        .map(|b| b.assets.clone())
        .unwrap_or_else(|| FixedBitSet::with_capacity(capacity));

      bundle_assets_bs.insert(*bundle_id, bs.clone());
      bs
    };

    // Computed ancestor-availability per node as a FixedBitSet.
    //
    // IMPORTANT: We cannot eagerly assign `ancestor_assets` during
    // propagation. Instead, we propagate bitsets, then assign `ancestor_assets` once at the end.
    //
    // Use a `Vec<Option<FixedBitSet>>` indexed by `NodeIndex::index()` to avoid hashing.
    let mut node_availability: Vec<Option<FixedBitSet>> = vec![None; g.node_bound()];

    for parent_node in &order {
      let parent_id = g
        .node_weight(*parent_node)
        .cloned()
        .context("parent node missing")?;

      let parent_ancestor_bs = node_availability[parent_node.index()].as_ref();

      let parent_is_isolated = ideal
        .get_bundle(&parent_id)
        .map(|b| {
          b.behavior == Some(BundleBehavior::Isolated)
            || b.behavior == Some(BundleBehavior::InlineIsolated)
        })
        .unwrap_or(false);

      // What this parent makes available to its children.
      // For isolated bundles, do not propagate ancestor availability.
      let mut parent_available = get_bundle_assets_bs(&parent_id);
      if !parent_is_isolated && let Some(parent_ancestor_bs) = parent_ancestor_bs {
        parent_available.union_with(parent_ancestor_bs);
      }

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
        let a_id = g
          .node_weight(*a)
          .map(|id| self.assets.id_for(id.as_asset_key()))
          .unwrap_or("");
        let b_id = g
          .node_weight(*b)
          .map(|id| self.assets.id_for(id.as_asset_key()))
          .unwrap_or("");
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
          .get_bundle(&child_id)
          .map(|b| {
            b.behavior == Some(BundleBehavior::Isolated)
              || b.behavior == Some(BundleBehavior::InlineIsolated)
          })
          .unwrap_or(false);

        if child_is_isolated {
          node_availability[child_node.index()] = Some(FixedBitSet::with_capacity(capacity));
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
        match &mut node_availability[child_node.index()] {
          Some(prev) => {
            prev.intersect_with(&current_available);
          }
          None => {
            node_availability[child_node.index()] = Some(current_available);
          }
        }

        // If this child is parallel, add its assets to parallel availability.
        if edge_type == IdealEdgeType::Parallel {
          let child_assets = get_bundle_assets_bs(&child_id);
          parallel_availability.union_with(&child_assets);
        }
      }
    }

    // Final pass: assign `ancestor_assets` once per bundle.
    for node in &order {
      let bundle_id = g
        .node_weight(*node)
        .cloned()
        .context("bundle node missing")?;

      let ancestor_bs = node_availability[node.index()].as_ref();

      let bundle = ideal.get_bundle_mut(&bundle_id).context("bundle missing")?;

      if bundle.behavior == Some(BundleBehavior::Isolated)
        || bundle.behavior == Some(BundleBehavior::InlineIsolated)
      {
        bundle.ancestor_assets.clear();
      } else {
        bundle.ancestor_assets = ancestor_bs.cloned().unwrap_or_default();
      }

      self.decision(
        "availability",
        super::types::DecisionKind::AvailabilityComputed {
          bundle_root: bundle_id.as_asset_key(),
          ancestor_assets_len: bundle.ancestor_assets.count_ones(..),
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

      // Assign availability to each bundle in the SCC.
      for &bundle_node in &sccs[scc_idx] {
        let bundle_id = g
          .node_weight(bundle_node)
          .cloned()
          .context("bundle node missing")?;

        let bundle = ideal.get_bundle_mut(&bundle_id).context("bundle missing")?;

        if bundle.behavior == Some(BundleBehavior::Isolated)
          || bundle.behavior == Some(BundleBehavior::InlineIsolated)
        {
          bundle.ancestor_assets.clear();
        } else {
          bundle.ancestor_assets = availability_bs.clone();
        }

        self.decision(
          "availability",
          super::types::DecisionKind::AvailabilityComputed {
            bundle_root: bundle_id.as_asset_key(),
            ancestor_assets_len: bundle.ancestor_assets.count_ones(..),
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
        if let Some(b) = ideal.get_bundle(&bundle_id) {
          scc_available.union_with(&self.bundle_available_bitset(b));
        }
      }

      available_for_scc[scc_idx] = scc_available;
    }

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

      // Duplicate asset into ALL reaching entry-like bundles (matching JS algorithm).
      // Each entry-like bundle must independently contain every sync-reachable asset
      // because they can be loaded in isolation.
      for &root in &reaching_entry_like {
        let bundle_id = &root_bundle_ids[&root];
        let bundle = ideal
          .get_bundle_mut(bundle_id)
          .context("entry-like bundle missing for duplication")?;
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

      // Track canonical bundle assignment (smallest entry-like for determinism).
      if !reaching_entry_like.is_empty() {
        let canonical = reaching_entry_like.iter().copied().min().unwrap();
        let bundle_id = root_bundle_ids[&canonical];
        // Only record canonical placement here (asset may have been duplicated into multiple bundles).
        // `move_asset_to_bundle` would remove it from the other entry-like bundles, which we don't want.
        ideal.set_asset_bundle(asset_key, Some(bundle_id));
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

      if splittable_roots.len() > 1 {
        // Multi-root asset with no entry-like roots -> deferred to Phase 8.
        continue;
      } else if splittable_roots.len() == 1 {
        // Single splittable root -> place directly.
        let root = splittable_roots[0];
        let bundle_id = &root_bundle_ids[&root];
        let bundle = ideal
          .get_bundle_mut(bundle_id)
          .context("bundle missing for single-root placement")?;
        bundle.assets.insert(asset_key.0 as usize);

        // If this asset was duplicated into one or more entry-like bundles, we must not
        // "move" it here (which would remove it from the canonical entry-like bundle).
        // Keep the canonical `asset_to_bundle` mapping stable in that case.
        if reaching_entry_like.is_empty() {
          ideal.move_asset_to_bundle(asset_key, bundle_id)?;
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
    reachability: &Reachability,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Precompute bundle ids for all known bundle roots once.
    //
    // This avoids repeated `self.assets.id_for(root).to_string()` allocations in hot loops.
    let root_bundle_ids: HashMap<AssetKey, IdealBundleId> = self
      .bundle_roots
      .iter()
      .chain(self.entry_like_roots.iter())
      .map(|&key| (key, IdealBundleId::from_asset_key(key)))
      .collect();

    let mut eligible_roots_by_asset: HashMap<AssetKey, Vec<AssetKey>> = HashMap::new();

    // Precompute bundle-edge lookups to avoid O(n^2) scans of `ideal.bundle_edges`.
    //
    // Map: from_bundle -> {to_bundle}
    let mut edge_targets: HashMap<IdealBundleId, HashSet<IdealBundleId>> = HashMap::new();

    // For parallel sibling checks.
    // Map: parent_bundle -> {parallel_child_bundle}
    let mut parallel_children: HashMap<IdealBundleId, HashSet<IdealBundleId>> = HashMap::new();
    // Map: parallel_child_bundle -> [parent_bundle]
    let mut parallel_parents_by_child: HashMap<IdealBundleId, Vec<IdealBundleId>> = HashMap::new();

    for (from, to, ty) in &ideal.bundle_edges {
      edge_targets.entry(*from).or_default().insert(*to);

      if *ty == IdealEdgeType::Parallel {
        parallel_children.entry(*from).or_default().insert(*to);
        parallel_parents_by_child
          .entry(*to)
          .or_default()
          .push(*from);
      }
    }

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

      if self.bundle_roots.contains(&asset_key) {
        continue;
      }

      // If this asset is reachable from any entry root, Phase 7 already duplicated it into
      // the entry bundle(s) and intentionally avoided placing it into splittable roots.
      // In that case, no shared bundle extraction is needed.
      if bs
        .ones()
        .map(|i| reachability.roots[i])
        .any(|r| self.entry_roots.contains(&r))
      {
        continue;
      }

      // Only process multi-root assets (those skipped in Phase 7).
      // Avoid allocating a Vec unless the asset is truly multi-root.
      let mut first: Option<AssetKey> = None;
      let mut second: Option<AssetKey> = None;
      let mut extra: Vec<AssetKey> = Vec::new();

      for r in bs.ones().map(|i| reachability.roots[i]) {
        if self.entry_like_roots.contains(&r) {
          continue;
        }

        match (first, second) {
          (None, _) => first = Some(r),
          (Some(_), None) => second = Some(r),
          (Some(_), Some(_)) => extra.push(r),
        }
      }

      let (Some(first), Some(second)) = (first, second) else {
        continue; // Not multi-root.
      };
      let mut splittable_roots: Vec<AssetKey> = Vec::with_capacity(2 + extra.len());
      splittable_roots.push(first);
      splittable_roots.push(second);
      splittable_roots.extend(extra);

      // Deterministic ordering is relied upon by the parallel sibling availability rule.
      splittable_roots.sort();
      splittable_roots.dedup();

      // Lookup bundle ids for roots once per asset.
      let splittable_root_ids: Vec<(AssetKey, &IdealBundleId)> = splittable_roots
        .iter()
        .copied()
        .map(|root| (root, &root_bundle_ids[&root]))
        .collect();

      // Filter by availability: only keep roots where asset is NOT already available.
      // An asset is considered available from root R if:
      // 1. It's in R's ancestor_assets (already in an ancestor bundle), OR
      // 2. Another root in the reachable set is an ancestor of R (the asset will
      //    be placed in that ancestor's bundle, making it available via bundle reuse).
      //
      // Performance notes:
      // - Check 2 (bundle reuse) is optimized from O(roots^2) to O(roots) by precomputing a
      //   set of all splittable root bundle ids and doing set membership checks.
      // - Check 3 (parallel sibling) is optimized similarly by incrementally tracking bundle
      //   ids for earlier roots (roots are deterministically sorted).
      let splittable_bundle_id_set: HashSet<&IdealBundleId> =
        splittable_root_ids.iter().map(|(_, id)| *id).collect();

      let mut earlier_sibling_bundle_ids: HashSet<&IdealBundleId> = HashSet::new();

      let mut eligible: Vec<AssetKey> = Vec::new();
      for (root, bundle_id) in &splittable_root_ids {
        let Some(bundle) = ideal.get_bundle(bundle_id) else {
          // Still treat this root as an "earlier sibling" for subsequent roots.
          earlier_sibling_bundle_ids.insert(*bundle_id);
          continue;
        };

        // Check 1: standard availability.
        if bundle.ancestor_assets.contains(asset_key.0 as usize) {
          earlier_sibling_bundle_ids.insert(*bundle_id);
          continue;
        }

        // Check 2: bundle reuse / subgraph detection.
        // If there's a bundle edge path from this root to another reachable root
        // that dominates the asset, the asset will be in that other root's bundle.
        // This root can load it via the edge rather than needing a shared bundle.
        let available_via_reuse = edge_targets.get(*bundle_id).is_some_and(|targets| {
          targets
            .iter()
            .any(|t| t != *bundle_id && splittable_bundle_id_set.contains(t))
        });

        if available_via_reuse {
          earlier_sibling_bundle_ids.insert(*bundle_id);
          continue;
        }

        // Check 3: parallel sibling availability.
        // If this root has a parallel sibling that also reaches the asset and
        // comes before it in deterministic ordering, the asset will be placed
        // in the earlier sibling's bundle and available via parallel loading.
        let available_via_parallel = parallel_parents_by_child
          .get(*bundle_id)
          .into_iter()
          .flatten()
          .any(|parent| {
            parallel_children.get(parent).is_some_and(|children| {
              children
                .iter()
                .any(|child| earlier_sibling_bundle_ids.contains(child))
            })
          });

        if available_via_parallel {
          earlier_sibling_bundle_ids.insert(*bundle_id);
          continue;
        }

        eligible.push(*root);
        earlier_sibling_bundle_ids.insert(*bundle_id);
      }

      if eligible.len() > 1 {
        eligible.sort();
        eligible.dedup();
        eligible_roots_by_asset.insert(asset_key, eligible);
      } else if eligible.len() == 1 {
        // Availability reduced to single root -> place directly.
        let root = eligible[0];
        let bundle_id = &root_bundle_ids[&root];
        let bundle = ideal
          .get_bundle_mut(bundle_id)
          .context("bundle missing for single-eligible placement")?;
        bundle.assets.insert(asset_key.0 as usize);
        ideal.move_asset_to_bundle(asset_key, bundle_id)?;
        self.decision(
          "placement",
          super::types::DecisionKind::AssetAssignedToBundle {
            asset: asset_key,
            bundle_root: root,
            reason: super::types::AssetAssignmentReason::SingleEligibleRoot,
          },
        );
      } else {
        // available from all roots -> no placement needed.
      }
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
    tracing::debug!(
      unique_root_sets = assets_by_root_set.len(),
      "create_shared_bundles: grouped assets by root set"
    );

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
        ancestor_assets: FixedBitSet::with_capacity(self.assets.len()),
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
        let in_ancestors = parent_bundle.ancestor_assets.contains(root_key.0 as usize);

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

    // Remove internalized bundles and place their assets into parent bundles.
    for (bundle_id, parents) in &internalized {
      // Collect the bundle's assets before removing it.
      let bundle_assets: Vec<usize> = ideal
        .get_bundle(bundle_id)
        .map(|b| b.assets.ones().collect())
        .unwrap_or_default();

      // Add the internalized bundle's assets to each parent bundle.
      for parent_id in parents {
        if let Some(parent_bundle) = ideal.get_bundle_mut(parent_id) {
          for idx in &bundle_assets {
            parent_bundle.assets.insert(*idx);
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
          || bundle.ancestor_assets.contains(asset_key.0 as usize))
      {
        return true;
      }

      // Check sync-reachability from this bundle's root asset using Phase 5 reachability.
      if let Some(asset_sync_node_idx) = asset_sync_node_idx {
        let from_root_key = id.as_asset_key();
        if let Some(&root_bit) = reachability.root_to_bit.get(&from_root_key)
          && reachability.reach_bits[asset_sync_node_idx.index()].contains(root_bit)
        {
          return true;
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
