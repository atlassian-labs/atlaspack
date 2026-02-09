use std::collections::{HashMap, HashSet};

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
use petgraph::{Direction, algo::dominators, graph::NodeIndex, stable_graph::StableDiGraph};
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

  // Bundle roots are either entry assets or boundary assets.
  bundle_roots: HashSet<super::types::AssetKey>,
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
    debug!(
      interned_assets = self.assets.ids.len(),
      "ideal graph: interned asset ids"
    );
    let entry_assets = self.extract_entry_assets(asset_graph)?;

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

    // Phase 4: asset placement (zero duplication).
    let mut ideal = self.assign_assets_to_bundles(asset_graph, &entry_assets)?;

    // Phase 5: bundle dependency graph (edge types).
    self.build_bundle_edges(asset_graph, &mut ideal)?;

    // Phase 6: availability propagation.
    self.compute_availability(&mut ideal)?;

    // Phase 7: reachability per bundle root (sync graph traversal).
    let reachability = self.compute_reachability(asset_graph, &ideal)?;

    // Phase 8: shared bundle extraction (rewrite ideal graph, still zero-duplication).
    self.create_shared_bundles(asset_graph, &reachability, &mut ideal)?;

    // Phase 9: availability propagation again after shared bundle insertion.
    self.compute_availability(&mut ideal)?;

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
        if let Some(asset) = asset_graph.get_asset(&maybe_entry_asset) {
          if let Some(key) = self.assets.key_for(&asset.id) {
            entries.push(key);
          }
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

  fn immediate_dominator_asset_key(&self, asset_key: AssetKey) -> anyhow::Result<Option<AssetKey>> {
    let Some(doms) = &self.dominators else {
      return Ok(None);
    };
    let Some(&node) = self.asset_to_sync_node.get(&asset_key) else {
      return Ok(None);
    };

    let idom = doms.immediate_dominator(node);
    let Some(idom) = idom else {
      return Ok(None);
    };

    let idom_node = self
      .sync_graph
      .node_weight(idom)
      .copied()
      .context("idom node missing in sync graph")?;

    match idom_node {
      SyncNode::VirtualRoot => Ok(None),
      SyncNode::Asset(k) => Ok(Some(k)),
    }
  }

  // ----------------------------
  // Phase 4: Placement
  // ----------------------------

  #[instrument(level = "debug", skip_all, fields(entries = entry_assets.len()))]
  fn assign_assets_to_bundles(
    &mut self,
    asset_graph: &AssetGraph,
    entry_assets: &[AssetKey],
  ) -> anyhow::Result<IdealGraph> {
    self.bundle_roots.clear();
    self.bundle_roots.extend(entry_assets.iter().cloned());
    self
      .bundle_roots
      .extend(self.bundle_boundaries.iter().cloned());

    let mut ideal = IdealGraph::default();

    // Create bundles for all bundle roots.
    for root_key in self.bundle_roots.clone() {
      let root_asset_id = self.assets.id_for(root_key).to_string();

      let asset = self
        .find_asset_by_id(asset_graph, &root_asset_id)
        .context("bundle root asset missing from graph")?;

      let bundle_id = IdealBundleId(root_asset_id.clone());

      ideal.bundles.insert(
        bundle_id.clone(),
        IdealBundle {
          id: bundle_id.clone(),
          root_asset_id: Some(root_asset_id.clone()),
          assets: HashSet::from([root_asset_id.clone()]),
          bundle_type: asset.file_type.clone(),
          // TODO: derive env/target from entries/graph once available.
          needs_stable_name: false,
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

    // Assign all other assets by walking up dominators until we hit a bundle root.
    for asset in asset_graph.get_assets() {
      if ideal.asset_to_bundle.contains_key(&asset.id) {
        continue;
      }

      let bundle_id = self.find_dominating_bundle_root(&asset.id)?;
      let bundle = ideal
        .bundles
        .get_mut(&bundle_id)
        .context("dominating bundle root missing")?;

      bundle.assets.insert(asset.id.clone());
      ideal
        .asset_to_bundle
        .insert(asset.id.clone(), bundle_id.clone());

      let asset_key = self
        .assets
        .key_for(&asset.id)
        .context("asset not interned")?;

      self.decision(
        "placement",
        super::types::DecisionKind::AssetAssignedToBundle {
          asset: asset_key,
          bundle_root: self
            .assets
            .key_for(&bundle_id.0)
            .context("bundle root not interned")?,
          reason: super::types::AssetAssignmentReason::Dominator,
        },
      );
    }

    debug!(
      bundles = ideal.bundles.len(),
      assets = ideal.asset_to_bundle.len(),
      "ideal graph: assigned assets to bundles"
    );
    Ok(ideal)
  }

  fn find_dominating_bundle_root(&self, asset_id: &str) -> anyhow::Result<IdealBundleId> {
    let Some(mut current) = self.assets.key_for(asset_id) else {
      anyhow::bail!("asset id not interned: {asset_id}");
    };

    loop {
      if self.bundle_roots.contains(&current) {
        return Ok(IdealBundleId(self.assets.id_for(current).to_string()));
      }

      let current_id = self.assets.id_for(current).to_string();

      let Some(idom) = self
        .immediate_dominator_asset_key(current)
        .with_context(|| format!("computing idom for {current_id}"))?
      else {
        // Asset is reachable from multiple roots (no single dominator).
        // Place it deterministically in the smallest bundle root for now.
        let fallback = self
          .bundle_roots
          .iter()
          .copied()
          .min()
          .context("no bundle roots")?;
        return Ok(IdealBundleId(self.assets.id_for(fallback).to_string()));
      };

      current = idom;
    }
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
  /// This is currently implemented with `HashSet<String>` for simplicity. We can swap this
  /// for a bitset representation once the algorithm is stable.
  #[instrument(level = "debug", skip_all)]
  fn compute_availability(&mut self, ideal: &mut IdealGraph) -> anyhow::Result<()> {
    // Build a petgraph representation for topo traversal.
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

    // Attempt topo sort; if cyclic, fall back to a conservative no-op.
    let order = match petgraph::algo::toposort(&g, None) {
      Ok(o) => o,
      Err(_) => {
        // TODO: implement SCC-based fixpoint propagation.
        debug!("ideal graph: bundle graph cyclic; skipping availability propagation");
        return Ok(());
      }
    };

    for node in order {
      let bundle_id = g
        .node_weight(node)
        .cloned()
        .context("bundle node missing")?;

      let parents: Vec<IdealBundleId> = g
        .neighbors_directed(node, Direction::Incoming)
        .filter_map(|p| g.node_weight(p).cloned())
        .collect();

      let mut availability: Option<HashSet<String>> = None;

      for parent in parents {
        let parent_assets = ideal
          .bundles
          .get(&parent)
          .map(|b| b.all_assets_available_from_here())
          .unwrap_or_default();

        availability = Some(match availability {
          None => parent_assets,
          Some(prev) => prev.intersection(&parent_assets).cloned().collect(),
        });
      }

      let availability = availability.unwrap_or_default();

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

    debug!(
      bundles = ideal.bundles.len(),
      "ideal graph: computed availability for bundles"
    );
    Ok(())
  }

  // ----------------------------
  // Phase 7: Reachability
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn compute_reachability(
    &mut self,
    asset_graph: &AssetGraph,
    _ideal: &IdealGraph,
  ) -> anyhow::Result<HashMap<AssetKey, HashSet<AssetKey>>> {
    // Reachable assets per bundle root (sync traversal only, per research doc).
    let mut reachability: HashMap<AssetKey, HashSet<AssetKey>> = HashMap::new();

    let roots: Vec<AssetKey> = self.bundle_roots.iter().copied().collect();

    for root_key in roots {
      let root_id = self.assets.id_for(root_key).to_string();
      let mut visited: HashSet<AssetKey> = HashSet::new();
      let mut stack: Vec<AssetKey> = vec![root_key];

      while let Some(cur) = stack.pop() {
        if !visited.insert(cur) {
          continue;
        }

        let cur_id = self.assets.id_for(cur);
        let Some(cur_node_id) = asset_graph.get_node_id_by_content_key(cur_id).copied() else {
          continue;
        };

        // Traverse asset -> dep -> asset edges, but only through sync deps, and do not traverse
        // past bundle boundaries.
        for dep_node in asset_graph.get_outgoing_neighbors(&cur_node_id) {
          let Some(dep) = asset_graph.get_dependency(&dep_node) else {
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

          for target_node in asset_graph.get_outgoing_neighbors(&dep_node) {
            let Some(target_asset) = asset_graph.get_asset(&target_node) else {
              continue;
            };

            let Some(target_key) = self.assets.key_for(&target_asset.id) else {
              continue;
            };

            if self.bundle_boundaries.contains(&target_key) {
              continue;
            }

            stack.push(target_key);
          }
        }
      }

      self.decision(
        "reachability",
        super::types::DecisionKind::ReachabilityComputed {
          bundle_root: root_key,
          reachable_assets_len: visited.len(),
        },
      );

      debug!(
        bundle_root = root_id,
        reachable = visited.len(),
        "ideal graph: reachability computed"
      );
      reachability.insert(root_key, visited);
    }

    // Ensure all roots have an entry.
    debug!(
      roots = reachability.len(),
      "ideal graph: computed reachability for roots"
    );
    Ok(reachability)
  }

  // ----------------------------
  // Phase 8: Shared bundle creation
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn create_shared_bundles(
    &mut self,
    _asset_graph: &AssetGraph,
    reachability: &HashMap<AssetKey, HashSet<AssetKey>>,
    ideal: &mut IdealGraph,
  ) -> anyhow::Result<()> {
    // Initial conservative implementation:
    // - For each asset, compute roots that reach it.
    // - Filter roots where the asset is already available via availability intersection.
    // - If >1 roots remain, create one shared bundle for that asset set and move assets.
    //
    // This will be refined to match the JS algorithm more closely.

    // 1) Invert reachability: asset -> roots.
    let mut roots_by_asset: HashMap<AssetKey, Vec<AssetKey>> = HashMap::new();
    for (root, reachable) in reachability {
      for &asset in reachable {
        roots_by_asset.entry(asset).or_default().push(*root);
      }
    }

    // 2) Filter by availability: only keep roots where asset is NOT already available.
    // Note: availability currently stored per bundle as `ancestor_assets` (strings). Convert to keys.
    let mut eligible_roots_by_asset: HashMap<AssetKey, Vec<AssetKey>> = HashMap::new();

    for (asset, roots) in roots_by_asset {
      // Never move bundle root assets into shared bundles.
      if self.bundle_roots.contains(&asset) {
        continue;
      }

      let asset_id = self.assets.id_for(asset);

      let mut eligible: Vec<AssetKey> = Vec::new();
      for root in roots {
        let root_id = self.assets.id_for(root);
        let bundle_id = super::types::IdealBundleId(root_id.to_string());
        let Some(bundle) = ideal.bundles.get(&bundle_id) else {
          continue;
        };

        // If already available when this bundle loads, skip.
        if bundle.ancestor_assets.contains(asset_id) {
          continue;
        }

        eligible.push(root);
      }

      if eligible.len() > 1 {
        eligible.sort();
        eligible.dedup();
        eligible_roots_by_asset.insert(asset, eligible);
      }
    }

    if eligible_roots_by_asset.is_empty() {
      debug!("ideal graph: no shared bundle candidates");
      return Ok(());
    }

    // 3) Group assets by eligible root set (so we create fewer shared bundles).
    let mut assets_by_root_set: HashMap<Vec<AssetKey>, Vec<AssetKey>> = HashMap::new();
    for (asset, roots) in eligible_roots_by_asset {
      assets_by_root_set.entry(roots).or_default().push(asset);
    }

    // 4) For each group, create a shared bundle and move assets into it.
    for (mut roots, assets) in assets_by_root_set {
      roots.sort();
      roots.dedup();

      // Shared bundle id: deterministic synthetic key derived from roots.
      // We allocate a new synthetic AssetKey by appending to `asset_ids` table.
      let shared_name = format!(
        "@@shared:{}",
        roots
          .iter()
          .map(|r| self.assets.id_for(*r))
          .collect::<Vec<_>>()
          .join(",")
      );

      // Extend interner (note: this changes key space only within this build).
      let shared_key = {
        let next = super::types::AssetKey(u32::try_from(self.assets.ids.len()).unwrap());
        self.assets.by_id.insert(shared_name.clone(), next);
        self.assets.ids.push(shared_name.clone());
        next
      };

      let shared_bundle_id = super::types::IdealBundleId(shared_name.clone());

      ideal.create_bundle(super::types::IdealBundle {
        id: shared_bundle_id.clone(),
        root_asset_id: None,
        assets: HashSet::new(),
        bundle_type: atlaspack_core::types::FileType::Js,
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

      // Move each asset into the shared bundle.
      for asset in assets {
        let asset_id = self.assets.id_for(asset).to_string();

        // Move to shared.
        let _prev = ideal.move_asset_to_bundle(&asset_id, &shared_bundle_id)?;

        // Record decisions for moves (use first root as a representative 'from').
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

      // Rewrite edges: retarget all incoming edges that pointed at any of the source roots
      // to instead point at the shared bundle.
      //
      // This prevents leaving stale/duplicate edges around.
      let roots_set: HashSet<IdealBundleId> = roots
        .iter()
        .map(|r| IdealBundleId(self.assets.id_for(*r).to_string()))
        .collect();

      let rewritten = ideal.retarget_incoming_edges(&roots_set, &shared_bundle_id);
      for (from, _old_to, _ty) in rewritten {
        let from_root = self
          .assets
          .key_for(&from.0)
          .unwrap_or(super::types::AssetKey(u32::MAX));

        self.decision(
          "shared",
          super::types::DecisionKind::BundleEdgeRewritten {
            from_bundle_root: from_root,
            to_bundle_root: shared_key,
            via_shared_bundle_root: shared_key,
          },
        );
      }

      // Ensure each source root depends on the shared bundle.
      // We add a Sync edge, and remove any existing edge to avoid duplicates.
      for root in roots {
        let from_id = IdealBundleId(self.assets.id_for(root).to_string());
        ideal.remove_bundle_edge(&from_id, &shared_bundle_id);
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
  // Helpers
  // ----------------------------

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
