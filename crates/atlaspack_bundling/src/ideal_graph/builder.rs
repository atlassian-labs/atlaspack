use std::collections::{HashMap, HashSet};

use anyhow::Context;
use atlaspack_core::{
  asset_graph::{AssetGraph, NodeId},
  types::{BundleBehavior, Priority},
};
use petgraph::{Direction, algo::dominators, graph::NodeIndex, stable_graph::StableDiGraph};
use tracing::{debug, instrument};

use super::types::{
  IdealBundle, IdealBundleId, IdealEdgeType, IdealGraph, IdealGraphBuildOptions,
  IdealGraphBuildStats,
};

/// Intermediate state for building an [`IdealGraph`].
///
/// This mirrors the phase-based design in `bundler-rust-rewrite-research.md`.
#[derive(Debug, Default)]
pub struct IdealGraphBuilder {
  pub options: IdealGraphBuildOptions,

  // Debugging
  decisions: Option<super::types::DecisionLog>,

  // Phase outputs / caches
  bundle_boundaries: HashSet<String>,

  sync_graph: StableDiGraph<String, ()>,
  virtual_root: Option<NodeIndex>,
  asset_to_sync_node: HashMap<String, NodeIndex>,

  dominators: Option<dominators::Dominators<NodeIndex>>,

  // Bundle roots are either entry assets or boundary assets.
  bundle_roots: HashSet<String>,
}

impl IdealGraphBuilder {
  fn decision(&mut self, phase: &'static str, kind: super::types::DecisionKind) {
    if let Some(log) = self.decisions.as_mut() {
      log.push(phase, kind);
    }
  }

  pub fn new(options: IdealGraphBuildOptions) -> Self {
    let decisions = options
      .collect_debug
      .then_some(super::types::DecisionLog::default());

    Self {
      options,
      decisions,
      ..Self::default()
    }
  }

  /// Full pipeline entrypoint.
  #[instrument(level = "debug", skip_all, fields(collect_debug = self.options.collect_debug))]
  pub fn build(
    mut self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<(IdealGraph, IdealGraphBuildStats)> {
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

    if let Some(decisions) = self.decisions.take() {
      ideal.debug = Some(super::types::IdealGraphDebug { decisions });
    }

    Ok((ideal, stats))
  }

  // ----------------------------
  // Phase 0: Entry extraction
  // ----------------------------

  #[instrument(level = "debug", skip_all)]
  fn extract_entry_assets(&self, asset_graph: &AssetGraph) -> anyhow::Result<Vec<String>> {
    let mut entries = Vec::new();

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
          entries.push(asset.id.clone());
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
            let inserted = self.bundle_boundaries.insert(target_asset.id.clone());
            if inserted {
              self.decision(
                "boundaries",
                super::types::DecisionKind::BoundaryCreated {
                  asset_id: target_asset.id.clone(),
                  from_asset_id: from_asset.id.clone(),
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
    entry_assets: &[String],
  ) -> anyhow::Result<()> {
    self.sync_graph = StableDiGraph::new();
    self.asset_to_sync_node.clear();

    let virtual_root = self.sync_graph.add_node("@@virtual_root".to_string());
    self.virtual_root = Some(virtual_root);

    // Add all assets as nodes.
    for asset in asset_graph.get_assets() {
      let idx = self.sync_graph.add_node(asset.id.clone());
      self.asset_to_sync_node.insert(asset.id.clone(), idx);
    }

    // Connect virtual root to entry assets.
    for entry in entry_assets {
      let Some(&entry_idx) = self.asset_to_sync_node.get(entry) else {
        continue;
      };
      self.sync_graph.add_edge(virtual_root, entry_idx, ());
    }

    // Add edges for sync dependencies only, stopping at bundle boundaries and isolated bundles.
    for asset_node_id in self.asset_node_ids(asset_graph) {
      let Some(from_asset) = asset_graph.get_asset(&asset_node_id) else {
        continue;
      };
      let Some(&from_idx) = self.asset_to_sync_node.get(&from_asset.id) else {
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

          if self.bundle_boundaries.contains(&target_asset.id) {
            continue;
          }

          let Some(&to_idx) = self.asset_to_sync_node.get(&target_asset.id) else {
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

  fn immediate_dominator_asset_id(&self, asset_id: &str) -> anyhow::Result<Option<String>> {
    let Some(doms) = &self.dominators else {
      return Ok(None);
    };
    let Some(&node) = self.asset_to_sync_node.get(asset_id) else {
      return Ok(None);
    };

    let idom = doms.immediate_dominator(node);
    let Some(idom) = idom else {
      return Ok(None);
    };

    let idom_asset = self
      .sync_graph
      .node_weight(idom)
      .cloned()
      .context("idom node missing in sync graph")?;

    if idom_asset == "@@virtual_root" {
      Ok(None)
    } else {
      Ok(Some(idom_asset))
    }
  }

  // ----------------------------
  // Phase 4: Placement
  // ----------------------------

  #[instrument(level = "debug", skip_all, fields(entries = entry_assets.len()))]
  fn assign_assets_to_bundles(
    &mut self,
    asset_graph: &AssetGraph,
    entry_assets: &[String],
  ) -> anyhow::Result<IdealGraph> {
    self.bundle_roots.clear();
    self.bundle_roots.extend(entry_assets.iter().cloned());
    self
      .bundle_roots
      .extend(self.bundle_boundaries.iter().cloned());

    let mut ideal = IdealGraph::default();

    // Create bundles for all bundle roots.
    for root_asset_id in self.bundle_roots.clone() {
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
          bundle_id: bundle_id.clone(),
          root_asset_id: root_asset_id.clone(),
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

      self.decision(
        "placement",
        super::types::DecisionKind::AssetAssignedToBundle {
          asset_id: asset.id.clone(),
          bundle_id: bundle_id.clone(),
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
    let mut current = asset_id.to_string();

    loop {
      if self.bundle_roots.contains(&current) {
        return Ok(IdealBundleId(current));
      }

      let Some(idom) = self
        .immediate_dominator_asset_id(&current)
        .with_context(|| format!("computing idom for {current}"))?
      else {
        anyhow::bail!("asset {asset_id} has no dominating bundle root");
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
          if !self.bundle_roots.contains(&target_asset.id) {
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
          bundle_id: bundle_id.clone(),
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
