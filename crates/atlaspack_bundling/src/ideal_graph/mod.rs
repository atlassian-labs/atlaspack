//! "Ideal graph" bundling algorithm scaffolding.
//!
//! This module is an initial landing zone for the algorithm described in
//! `bundler-rust-rewrite-research.md` ("ideal graph").
//!
//! Goals of this scaffolding:
//! - Provide stable, testable Rust types to iterate on.
//! - Keep algorithm phases explicit (build graph, compute boundaries, dominators, placement, etc.).
//! - Avoid coupling to Parcel/JS implementation details so we can evolve safely.

pub mod builder;
pub mod query;
pub mod types;

mod dense_bitset;

use anyhow::Context;
use atlaspack_core::{
  asset_graph::AssetGraph,
  bundle_graph::{
    NativeBundleGraph,
    native_bundle_graph::{NativeBundleGraphEdgeType, NativeBundleGraphNode},
  },
};
use tracing::instrument;

use crate::Bundler;

use self::{
  builder::IdealGraphBuilder,
  types::{IdealGraph, IdealGraphBuildOptions, IdealGraphBuildStats},
};

fn is_boundary_behavior(behavior: Option<atlaspack_core::types::BundleBehavior>) -> bool {
  behavior.is_some_and(|b| b.is_boundary())
}

/// Bundler implementation backed by the (future) ideal graph algorithm.
///
/// For now this is a no-op scaffolding that builds an [`IdealGraph`] from the asset graph
/// and emits a minimal set of invariants into the provided [`NativeBundleGraph`].
#[derive(Debug, Default)]
pub struct IdealGraphBundler {
  pub options: IdealGraphBuildOptions,
}

impl IdealGraphBundler {
  pub fn new(options: IdealGraphBuildOptions) -> Self {
    Self { options }
  }

  /// Builds the intermediate ideal graph representation.
  #[instrument(level = "info", skip_all)]
  pub fn build_ideal_graph(
    &self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<(IdealGraph, IdealGraphBuildStats, types::BundlingReport)> {
    IdealGraphBuilder::new(self.options.clone())
      .build(asset_graph)
      .context("building IdealGraph via IdealGraphBuilder")
  }
}

impl Bundler for IdealGraphBundler {
  #[instrument(level = "info", skip_all)]
  fn bundle(
    &self,
    asset_graph: &AssetGraph,
    bundle_graph: &mut NativeBundleGraph,
  ) -> anyhow::Result<()> {
    let (ideal_graph, _stats, report) = self.build_ideal_graph(asset_graph)?;

    // Dump bundling decisions to a JSON file if requested via env var.
    if let Ok(dump_path) = std::env::var("ATLASPACK_DUMP_BUNDLER_DECISIONS") {
      let json = serde_json::to_string_pretty(&report)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
      let path = std::path::Path::new(&dump_path);
      if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
      }
      if let Err(e) = std::fs::write(path, &json) {
        tracing::warn!("Failed to dump bundler decisions to {}: {}", dump_path, e);
      } else {
        tracing::info!("Dumped bundler decisions to {}", dump_path);
      }
    }

    // Write formatted bundling report to file if requested.
    if let Ok(report_path) = std::env::var("ATLASPACK_BUNDLER_REPORT") {
      let t = std::time::Instant::now();
      let formatted = query::format_full_report(&report, &ideal_graph.assets);
      let format_ms = t.elapsed().as_secs_f64() * 1000.0;
      let output = format!("{}\n(Report formatted in {:.1}ms)\n", formatted, format_ms);
      let path = std::path::Path::new(&report_path);
      if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
      }
      if let Err(e) = std::fs::write(path, &output) {
        tracing::warn!("Failed to write bundler report to {}: {}", report_path, e);
      } else {
        tracing::info!("Wrote bundler report to {}", report_path);
      }
    }

    let root_node_id = *bundle_graph
      .get_node_id_by_content_key("@@root")
      .context("missing @@root node in NativeBundleGraph")?;

    use std::collections::HashMap;

    // Build an index of boundary dependencies by their resolved target asset id.
    //
    // Several places below need to find “incoming deps targeting this asset”.
    // Scanning all dependencies for each bundle is O(N*M) and can be billions of iterations.
    let mut boundary_deps_by_target_asset_id: HashMap<
      String,
      Vec<&atlaspack_core::types::Dependency>,
    > = HashMap::new();

    for dep in asset_graph.get_dependencies() {
      if dep.is_entry {
        continue;
      }

      let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dep.id) else {
        continue;
      };

      // Non-sync deps always create bundle groups. Sync deps normally don't, BUT sync deps
      // that target an asset with bundleBehavior=Isolated/InlineIsolated should also get bundle
      // groups so the isolated bundle is discoverable via `traverseBundles()`. This matches the
      // JS bundler which creates bundle groups for isolated assets regardless of dep priority.
      if dep.priority == atlaspack_core::types::Priority::Sync {
        for neighbor in asset_graph.get_outgoing_neighbors(dep_node_id) {
          if let Some(target_asset) = asset_graph.get_asset(&neighbor)
            && is_boundary_behavior(target_asset.bundle_behavior)
          {
            boundary_deps_by_target_asset_id
              .entry(target_asset.id.clone())
              .or_default()
              .push(dep);
          }
        }
        continue;
      }

      for neighbor in asset_graph.get_outgoing_neighbors(dep_node_id) {
        if let Some(target_asset) = asset_graph.get_asset(&neighbor) {
          boundary_deps_by_target_asset_id
            .entry(target_asset.id.clone())
            .or_default()
            .push(dep);
        }
      }
    }

    // Collect entry dependencies and their target assets.
    let entries: Vec<(atlaspack_core::types::Dependency, String)> = asset_graph
      .get_dependencies()
      .filter(|dep| dep.is_entry)
      .filter_map(|dep| {
        let dep_node_id = asset_graph.get_node_id_by_content_key(&dep.id)?;
        for neighbor in asset_graph.get_outgoing_neighbors(dep_node_id) {
          if let Some(asset) = asset_graph.get_asset(&neighbor) {
            return Some((dep.clone(), asset.id.clone()));
          }
        }
        None
      })
      .collect();

    anyhow::ensure!(!entries.is_empty(), "missing entry dependency");

    // Create bundle groups and bundles for each entry, then wire up the ideal graph bundles.
    use atlaspack_core::types::Target;

    // Track which ideal bundles have been materialized into the NativeBundleGraph.
    let mut materialized_bundle_nodes: HashMap<types::IdealBundleId, usize> = HashMap::new();

    for (entry_dep, entry_asset_id) in &entries {
      let target: Target = entry_dep.target.as_deref().cloned().unwrap_or_default();

      // Find the ideal bundle that contains this entry asset.
      let entry_asset_key = ideal_graph
        .assets
        .key_for(entry_asset_id)
        .context("entry asset missing from AssetInterner")?;

      let ideal_bundle_id = ideal_graph
        .asset_bundle(&entry_asset_key)
        .context("entry asset not assigned to any ideal bundle")?;

      let ideal_bundle = ideal_graph
        .get_bundle(&ideal_bundle_id)
        .context("ideal bundle missing")?;

      // Create the bundle group.
      let bundle_group_id = bg_key(&target.name, entry_asset_id);
      let bundle_group_node_id =
        bundle_graph.add_bundle_group(bundle_group_id, target.clone(), entry_asset_id.clone());

      // Materialize this ideal bundle into the NativeBundleGraph if not already done.
      let bundle_node_id = materialize_ideal_bundle(
        bundle_graph,
        &ideal_graph,
        &ideal_bundle_id,
        ideal_bundle,
        &target,
        entry_dep,
        &mut materialized_bundle_nodes,
        asset_graph,
      )?;

      // Wire up: root -> bundle_group, entry_dep -> bundle_group, bundle_group -> bundle.
      let entry_dep_node_id = *bundle_graph
        .get_node_id_by_content_key(&entry_dep.id)
        .ok_or_else(|| anyhow::anyhow!("Missing entry dependency node id"))?;

      let entry_asset_node_id = *bundle_graph
        .get_node_id_by_content_key(entry_asset_id)
        .ok_or_else(|| anyhow::anyhow!("Missing entry asset node id"))?;

      bundle_graph.add_edge(
        &root_node_id,
        &bundle_group_node_id,
        NativeBundleGraphEdgeType::Bundle,
      );
      bundle_graph.add_edge(
        &entry_dep_node_id,
        &bundle_group_node_id,
        NativeBundleGraphEdgeType::Null,
      );
      bundle_graph.add_edge(
        &bundle_group_node_id,
        &bundle_node_id,
        NativeBundleGraphEdgeType::Null,
      );
      bundle_graph.add_edge(
        &bundle_group_node_id,
        &bundle_node_id,
        NativeBundleGraphEdgeType::Bundle,
      );
      bundle_graph.add_edge(
        &bundle_node_id,
        &entry_asset_node_id,
        NativeBundleGraphEdgeType::Null,
      );
      bundle_graph.add_edge(
        &entry_dep_node_id,
        &entry_asset_node_id,
        NativeBundleGraphEdgeType::References,
      );
    }

    // Materialize any remaining ideal bundles not yet created (e.g. async/shared bundles)
    // and create bundle groups for async boundary bundles.
    //
    // NOTE: Sync type-change bundles (e.g. JS importing CSS) must NOT get their own
    // bundle group. They are materialized as sibling bundles in the *same* bundle group
    // as their parent bundle, and connected via `References` bundle-to-bundle edges.
    let default_target = entries
      .first()
      .and_then(|(dep, _)| dep.target.as_deref().cloned())
      .unwrap_or_default();

    let default_entry_dep = entries.first().map(|(dep, _)| dep).unwrap();

    for (ideal_bundle_id, ideal_bundle) in ideal_graph.bundles_iter() {
      if materialized_bundle_nodes.contains_key(ideal_bundle_id) {
        continue;
      }

      // Skip internalized bundles — they should not be materialized into the
      // bundle graph. InternalAsync edges from parent bundles handle resolution.
      if ideal_graph
        .internalized_bundles
        .contains_key(ideal_bundle_id)
      {
        continue;
      }

      let bundle_node_id = materialize_ideal_bundle(
        bundle_graph,
        &ideal_graph,
        ideal_bundle_id,
        ideal_bundle,
        &default_target,
        default_entry_dep,
        &mut materialized_bundle_nodes,
        asset_graph,
      )?;

      // Create a bundle group only for async boundary bundles (not shared bundles, and not
      // sync type-change bundles).
      //
      // A bundle gets its own bundle group if:
      // 1. The builder assigned it (bundle_group_root == self), OR
      // 2. It's an async boundary root not covered by the builder (fallback for
      //    edge cases where build_bundle_edges doesn't discover all lazy targets,
      //    e.g. when the source asset isn't in asset_to_containing_bundle).
      let ideal_bundle_asset_key = ideal_bundle_id.as_asset_key();
      let builder_assigned_own_group =
        ideal_bundle.bundle_group_root == Some(ideal_bundle_asset_key);
      let is_async_fallback = !builder_assigned_own_group
        && ideal_bundle.bundle_group_root.is_none()
        && ideal_bundle.behavior != Some(atlaspack_core::types::BundleBehavior::Inline)
        && ideal_bundle.root_asset_id.as_deref().is_some_and(|id| {
          boundary_deps_by_target_asset_id
            .get(id)
            .is_some_and(|deps| !deps.is_empty())
        });
      let has_own_bundle_group = builder_assigned_own_group || is_async_fallback;

      if let Some(root_asset_id) = &ideal_bundle.root_asset_id
        && has_own_bundle_group
      {
        let bundle_group_id = bg_key(&default_target.name, root_asset_id);
        let bundle_group_node_id = bundle_graph.add_bundle_group(
          bundle_group_id,
          default_target.clone(),
          root_asset_id.clone(),
        );

        // Find the dependency that leads to this bundle root.
        // We look for dependencies targeting this asset from the asset graph.
        //
        // IMPORTANT: only non-sync dependencies should connect to the async bundle group.
        // Sync deps to an async-root bundle should be represented via bundle-to-bundle
        // `References` edges, not via dependency -> bundle_group edges.
        let mut found_dep = false;
        if let Some(deps) = boundary_deps_by_target_asset_id.get(root_asset_id) {
          for dep in deps {
            if let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id) {
              bundle_graph.add_edge(
                &dep_bg_node_id,
                &bundle_group_node_id,
                NativeBundleGraphEdgeType::Null,
              );
              found_dep = true;
            }
          }
        }

        bundle_graph.add_edge(
          &bundle_group_node_id,
          &bundle_node_id,
          NativeBundleGraphEdgeType::Null,
        );
        bundle_graph.add_edge(
          &bundle_group_node_id,
          &bundle_node_id,
          NativeBundleGraphEdgeType::Bundle,
        );

        if let Some(&root_asset_node_id) = bundle_graph.get_node_id_by_content_key(root_asset_id) {
          bundle_graph.add_edge(
            &bundle_node_id,
            &root_asset_node_id,
            NativeBundleGraphEdgeType::Null,
          );
          if found_dep {
            // Add References edge from dep to asset for non-entry bundles too.
            if let Some(deps) = boundary_deps_by_target_asset_id.get(root_asset_id) {
              for dep in deps {
                if let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id) {
                  bundle_graph.add_edge(
                    &dep_bg_node_id,
                    &root_asset_node_id,
                    NativeBundleGraphEdgeType::References,
                  );
                }
              }
            }
          }
        }
      }
    }

    // Add inherited-group bundles to their parent's bundle group.
    // These are bundles where bundle_group_root points to a DIFFERENT bundle's root key,
    // meaning they should be members of that bundle's group (e.g. render.js inheriting
    // from dashboard-spa-container.html's bundle group).
    for (ideal_bundle_id, ideal_bundle) in ideal_graph.bundles_iter() {
      let bundle_asset_key = ideal_bundle_id.as_asset_key();

      // Only process bundles that inherit a parent's group.
      let Some(group_root_key) = ideal_bundle.bundle_group_root else {
        continue;
      };
      if group_root_key == bundle_asset_key {
        continue; // has its own group, already handled
      }

      let Some(&bundle_node_id) = materialized_bundle_nodes.get(ideal_bundle_id) else {
        continue;
      };

      let group_root_id = ideal_graph.assets.id_for(group_root_key);
      let bg_key = bg_key(&default_target.name, group_root_id);
      let Some(&bg_node_id) = bundle_graph.get_node_id_by_content_key(&bg_key) else {
        continue;
      };

      bundle_graph.add_edge(
        &bg_node_id,
        &bundle_node_id,
        NativeBundleGraphEdgeType::Null,
      );
      bundle_graph.add_edge(
        &bg_node_id,
        &bundle_node_id,
        NativeBundleGraphEdgeType::Bundle,
      );
    }

    // Wire shared bundles into the bundle groups of the async bundles that depend on them.
    // Shared bundles don't have their own bundle groups - they're siblings in existing groups.
    //
    // Two kinds of edges are created to match the JS bundler's `decorateLegacyGraph`:
    // 1. bundle_group -> shared_bundle (Null + Bundle): makes the shared bundle a member of
    //    the bundle group, equivalent to JS `addBundleToBundleGroup`.
    // 2. from_bundle -> shared_bundle (References): creates a bundle-to-bundle reference,
    //    equivalent to JS `createBundleReference`. This is critical for correct
    //    `isAssetReachableFromBundle` calculations — without it, the reachability traversal
    //    follows a different ancestry path and incorrectly skips runtime helper assets.
    for (ideal_bundle_id, ideal_bundle) in ideal_graph.bundles_iter() {
      if ideal_bundle.root_asset_id.is_some() {
        continue; // Not a shared bundle.
      }
      // Type-change sibling bundles (@@typechange:...) also have root_asset_id == None,
      // but they are NOT shared bundles. They are wired via bundle_edges instead.
      if ideal_graph
        .assets
        .id_for(ideal_bundle_id.as_asset_key())
        .starts_with("@@typechange:")
      {
        continue;
      }

      let Some(&shared_bundle_node_id) = materialized_bundle_nodes.get(ideal_bundle_id) else {
        continue;
      };

      // Find which bundles depend on this shared bundle via bundle_edges.
      for (from_id, to_id, _edge_type) in &ideal_graph.bundle_edges {
        if to_id != ideal_bundle_id {
          continue;
        }

        // `from_id` is a bundle that depends on this shared bundle.
        // Find that bundle's root asset and its bundle group.
        if let Some(from_bundle) = ideal_graph.get_bundle(from_id)
          && let Some(from_root_asset_id) = &from_bundle.root_asset_id
        {
          let bg_key = bg_key(&default_target.name, from_root_asset_id);
          if let Some(&bg_node_id) = bundle_graph.get_node_id_by_content_key(&bg_key) {
            // Add the shared bundle to the bundle group for graph traversal only (Null edge).
            //
            // IMPORTANT: Do NOT add a Bundle edge here. The JS bundler never calls
            // `addBundleToBundleGroup` for shared bundles — they are connected only via
            // `createBundleReference` (References edges from parent bundles). Adding a
            // Bundle edge would make the shared bundle a direct "member" of the bundle
            // group, which changes `isAssetReachableFromBundle` ancestry traversal and
            // causes runtime helper assets to be incorrectly skipped.
            bundle_graph.add_edge(
              &bg_node_id,
              &shared_bundle_node_id,
              NativeBundleGraphEdgeType::Null,
            );
          }

          // Add References edge from the parent bundle to the shared bundle.
          // This matches the JS bundler's createBundleReference behavior.
          if let Some(&from_bundle_node_id) = materialized_bundle_nodes.get(from_id) {
            bundle_graph.add_edge(
              &from_bundle_node_id,
              &shared_bundle_node_id,
              NativeBundleGraphEdgeType::References,
            );
          }
        }
      }
    }

    // Materialize ideal_graph.bundle_edges.
    //
    // For async boundaries, we connect bundle -> bundle_group with Bundle(3).
    // For sync boundaries:
    // - If the boundary is a *type change* (bundle types differ), connect bundle -> bundle
    //   with References(4), and add the child bundle into the *same* bundle group as the parent.
    // - Otherwise (sync edge to an existing async root), connect bundle -> bundle_group with
    //   References(4) (sibling/reference relationship, but the child still has its own group).
    {
      use std::collections::HashSet;

      let mut seen: HashSet<(usize, usize, NativeBundleGraphEdgeType)> = HashSet::new();

      for (from_id, to_id, edge_type) in &ideal_graph.bundle_edges {
        let Some(&from_bundle_node_id) = materialized_bundle_nodes.get(from_id) else {
          continue;
        };

        let Some(from_bundle) = ideal_graph.get_bundle(from_id) else {
          continue;
        };

        let Some(to_bundle) = ideal_graph.get_bundle(to_id) else {
          continue;
        };

        let is_sync_type_change = *edge_type == types::IdealEdgeType::Sync
          && from_bundle.bundle_type != to_bundle.bundle_type;

        if is_sync_type_change {
          // Sync type-change edges include type-change sibling bundles (@@typechange:*), which
          // have `root_asset_id == None`. They are siblings in the parent's bundle group.

          // Ensure bundle-to-bundle References edge exists.
          let Some(&to_bundle_node_id) = materialized_bundle_nodes.get(to_id) else {
            continue;
          };

          if seen.insert((
            from_bundle_node_id,
            to_bundle_node_id,
            NativeBundleGraphEdgeType::References,
          )) {
            bundle_graph.add_edge(
              &from_bundle_node_id,
              &to_bundle_node_id,
              NativeBundleGraphEdgeType::References,
            );
          }

          // Add the type-change bundle to the parent's bundle group.
          // Use bundle_group_root to find the correct bundle group for the parent.
          // This handles cases where the parent bundle inherits a group from an ancestor
          // (e.g. a JS bundle that is a sync type-change child of an HTML entry).
          let Some(bg_root_key) = from_bundle.bundle_group_root else {
            continue;
          };
          let bg_root_asset_id = ideal_graph.assets.id_for(bg_root_key);
          let from_bg_key = bg_key(&default_target.name, bg_root_asset_id);
          let Some(&from_bg_node_id) = bundle_graph.get_node_id_by_content_key(&from_bg_key) else {
            continue;
          };

          if seen.insert((
            from_bg_node_id,
            to_bundle_node_id,
            NativeBundleGraphEdgeType::Bundle,
          )) {
            bundle_graph.add_edge(
              &from_bg_node_id,
              &to_bundle_node_id,
              NativeBundleGraphEdgeType::Null,
            );
            bundle_graph.add_edge(
              &from_bg_node_id,
              &to_bundle_node_id,
              NativeBundleGraphEdgeType::Bundle,
            );
          }

          continue;
        }

        let Some(to_root_asset_id) = &to_bundle.root_asset_id else {
          continue; // Shared bundles don't have a bundle group.
        };

        // Non-type-change edges: connect to the child's bundle group (if it exists).
        let bg_key = bg_key(&default_target.name, to_root_asset_id);
        let Some(&to_bg_node_id) = bundle_graph.get_node_id_by_content_key(&bg_key) else {
          continue;
        };

        if *edge_type == types::IdealEdgeType::Sync {
          // Sync edge to an existing async bundle root.
          //
          // We must NOT add the target bundle into the source bundle group. Doing so makes the
          // runtime think the source bundle group contains multiple bundles that may need to be
          // loaded together, which injects loader runtimes (e.g. bundle-url.js) into the source.
          //
          // The target bundle already belongs to its own async bundle group and will be loaded by
          // whoever triggers that async boundary.
          if let Some(&to_bundle_node_id) = materialized_bundle_nodes.get(to_id)
            && seen.insert((
              from_bundle_node_id,
              to_bundle_node_id,
              NativeBundleGraphEdgeType::References,
            ))
          {
            bundle_graph.add_edge(
              &from_bundle_node_id,
              &to_bundle_node_id,
              NativeBundleGraphEdgeType::References,
            );
          }

          continue;
        }

        // Non-sync edges: connect to the child's bundle group (if it exists).
        let native_edge_type = NativeBundleGraphEdgeType::Bundle;

        if seen.insert((from_bundle_node_id, to_bg_node_id, native_edge_type)) {
          bundle_graph.add_edge(&from_bundle_node_id, &to_bg_node_id, native_edge_type);
        }
      }
    }

    // Add Contains edges by traversing the asset graph from each bundle's root asset,
    // following sync deps and stopping at bundle boundaries. This matches the behavior
    // of V2's addAssetGraphToBundle which duplicates shared sync assets into each
    // entry bundle rather than using zero-duplication placement.
    use std::collections::HashSet;

    // Dedup `bundle -> bundle_group` edges we add while materializing Contains edges.
    //
    // This is needed for shared/async bundles because `ideal_graph.bundle_edges` only
    // contains edges between ideal bundle roots. Shared bundles are not bundle roots,
    // so we must still ensure lazy deps inside them wire up their target bundle groups.
    let mut seen_bundle_to_bg_edges: HashSet<(usize, usize, NativeBundleGraphEdgeType)> =
      HashSet::new();

    for (ideal_bundle_id, ideal_bundle) in ideal_graph.bundles_iter() {
      let Some(&bundle_node_id) = materialized_bundle_nodes.get(ideal_bundle_id) else {
        continue;
      };
      // Determine if this is an entry bundle (needs sync-dep traversal to duplicate
      // shared assets like esmodule-helpers.js) or a non-entry bundle (should only
      // contain assets from the ideal graph's zero-duplication placement).
      let is_entry_bundle = entries.iter().any(|(_, eid)| {
        ideal_bundle
          .root_asset_id
          .as_ref()
          .map(|rid| rid == eid)
          .unwrap_or(false)
      });

      if is_entry_bundle {
        // Entry bundles: traverse the asset graph from the root, following sync deps,
        // stopping at bundle boundaries. This duplicates shared sync assets (like
        // esmodule-helpers.js) into each entry bundle, matching V2 behavior.
        let root_asset_id = match &ideal_bundle.root_asset_id {
          Some(id) => id.clone(),
          None => continue,
        };

        let mut visited_assets: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = vec![root_asset_id];

        while let Some(asset_id) = stack.pop() {
          if !visited_assets.insert(asset_id.clone()) {
            continue;
          }

          // Add Contains + Null edges for this asset.
          if let Some(&asset_node_id) = bundle_graph.get_node_id_by_content_key(&asset_id) {
            bundle_graph.add_edge(
              &bundle_node_id,
              &asset_node_id,
              NativeBundleGraphEdgeType::Contains,
            );
            bundle_graph.add_edge(
              &bundle_node_id,
              &asset_node_id,
              NativeBundleGraphEdgeType::Null,
            );
          }

          // Traverse outgoing deps from this asset.
          let Some(ag_node_id) = asset_graph.get_node_id_by_content_key(&asset_id) else {
            continue;
          };

          for dep_node_id in asset_graph.get_outgoing_neighbors(ag_node_id) {
            let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
              continue;
            };

            // Add Contains edge for the dependency itself.
            if let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id) {
              bundle_graph.add_edge(
                &bundle_node_id,
                &dep_bg_node_id,
                NativeBundleGraphEdgeType::Contains,
              );
            }

            // Cross-bundle edges from a bundle to a bundle group are materialized from
            // `ideal_graph.bundle_edges` (see above). When traversing assets for Contains
            // edges, we only need to stop at async deps (non-sync) and not traverse them.
            if dep.priority != atlaspack_core::types::Priority::Sync {
              continue;
            }

            // Check if the dep OR any target asset has Isolated/InlineIsolated behavior.
            // The bundleBehavior can be on the dependency itself or on the target asset.
            let dep_is_boundary = is_boundary_behavior(dep.bundle_behavior);
            let targets_boundary_asset = !dep_is_boundary
              && asset_graph
                .get_outgoing_neighbors(&dep_node_id)
                .iter()
                .any(|n| {
                  asset_graph
                    .get_asset(n)
                    .is_some_and(|a| is_boundary_behavior(a.bundle_behavior))
                });

            if dep_is_boundary || targets_boundary_asset {
              // Sync isolated/inline deps are boundaries. For Isolated and InlineIsolated
              // targets, the isolated asset has its own bundle group and we need to create
              // a `bundle -> bundle_group` edge so it's reachable via `traverseBundles()`.
              // Inline targets don't have bundle groups (they're embedded inline), so the
              // bg_key lookup below will simply not find them — no edge is created.
              for target_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
                let Some(target_asset) = asset_graph.get_asset(&target_node_id) else {
                  continue;
                };

                let target_bg_key = bg_key(&default_target.name, &target_asset.id);
                let Some(&to_bg_node_id) = bundle_graph.get_node_id_by_content_key(&target_bg_key)
                else {
                  continue;
                };

                let native_edge_type = NativeBundleGraphEdgeType::Bundle;
                if seen_bundle_to_bg_edges.insert((bundle_node_id, to_bg_node_id, native_edge_type))
                {
                  bundle_graph.add_edge(&bundle_node_id, &to_bg_node_id, native_edge_type);
                }
              }

              continue;
            }

            // Follow deps to target assets.
            for target_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
              if let Some(target_asset) = asset_graph.get_asset(&target_node_id) {
                // Don't traverse across sync type-changes when building entry bundle contents.
                //
                // Entry bundles compute their asset set by walking the sync subgraph in the asset graph.
                // However, sync type-change assets (e.g. JS -> CSS via `import './style.css'`) are
                // separated into type-change sibling bundles during ideal-graph placement.
                // If we traverse into them here, we incorrectly add the CSS asset into the JS bundle
                // as well, causing the asset to end up in both bundles.
                if target_asset.file_type != ideal_bundle.bundle_type {
                  continue;
                }

                // Don't cross into other bundle roots.
                let target_key = ideal_graph.assets.key_for(&target_asset.id);
                let is_other_bundle_root = target_key.is_some_and(|k| {
                  let target_bundle_id = types::IdealBundleId::from_asset_key(k);
                  ideal_graph.get_bundle(&target_bundle_id).is_some()
                    && target_bundle_id != *ideal_bundle_id
                });

                if is_other_bundle_root {
                  continue;
                }

                stack.push(target_asset.id.clone());
              }
            }
          }
        }
      } else {
        // Non-entry bundles (async, shared): use the ideal graph's asset set directly,
        // but do not duplicate assets that are already available from ancestor bundles.
        //
        // `IdealBundle.assets` may still include assets that also appear in ancestor bundles
        // (e.g. due to conservative placement). `ancestor_assets` is the canonical source of
        // availability, so we skip any assets that are already available.
        let is_type_change_bundle = ideal_graph
          .assets
          .id_for(ideal_bundle_id.as_asset_key())
          .starts_with("@@typechange:");

        for key_idx in ideal_bundle.assets.ones() {
          let asset_key = types::AssetKey(key_idx as u32);
          // Type-change sibling bundles are loaded as siblings of their parent bundle,
          // so they must still contain their own assets even if those assets are also
          // considered "available" via availability propagation.
          if !is_type_change_bundle && ideal_bundle.ancestor_assets.contains(key_idx as u32) {
            continue;
          }

          let asset_id = ideal_graph.assets.id_for(asset_key);
          if let Some(&asset_node_id) = bundle_graph.get_node_id_by_content_key(asset_id) {
            bundle_graph.add_edge(
              &bundle_node_id,
              &asset_node_id,
              NativeBundleGraphEdgeType::Contains,
            );
            bundle_graph.add_edge(
              &bundle_node_id,
              &asset_node_id,
              NativeBundleGraphEdgeType::Null,
            );
          }
        }

        // Add Contains edges for dependencies of assets in this bundle.
        for key_idx in ideal_bundle.assets.ones() {
          let asset_key = types::AssetKey(key_idx as u32);
          let asset_id = ideal_graph.assets.id_for(asset_key);
          if let Some(ag_node_id) = asset_graph.get_node_id_by_content_key(asset_id) {
            for dep_node_id in asset_graph.get_outgoing_neighbors(ag_node_id) {
              let Some(dep) = asset_graph.get_dependency(&dep_node_id) else {
                continue;
              };

              // Add Contains edge for the dependency itself.
              if let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id) {
                bundle_graph.add_edge(
                  &bundle_node_id,
                  &dep_bg_node_id,
                  NativeBundleGraphEdgeType::Contains,
                );
              }

              // Shared bundles are not ideal bundle roots and therefore don't appear in
              // `ideal_graph.bundle_edges`. If a shared/async bundle contains a lazy
              // dependency (or a sync isolated dependency), we must still add the parent
              // `bundle -> bundle_group` edge so the target bundle group is reachable from the root.
              let dep_has_boundary = is_boundary_behavior(dep.bundle_behavior);
              let target_has_boundary = !dep_has_boundary
                && dep.priority == atlaspack_core::types::Priority::Sync
                && asset_graph
                  .get_outgoing_neighbors(&dep_node_id)
                  .iter()
                  .any(|n| {
                    asset_graph
                      .get_asset(n)
                      .is_some_and(|a| is_boundary_behavior(a.bundle_behavior))
                  });
              let is_isolated_sync = dep.priority == atlaspack_core::types::Priority::Sync
                && (dep_has_boundary || target_has_boundary);

              if dep.priority == atlaspack_core::types::Priority::Lazy || is_isolated_sync {
                // Follow dep -> target asset (if resolved).
                for target_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
                  let Some(target_asset) = asset_graph.get_asset(&target_node_id) else {
                    continue;
                  };

                  // For isolated sync deps, the `bundle_behavior` may live on the *asset* rather
                  // than the dependency. Ensure we still treat it as a boundary.
                  if dep.priority == atlaspack_core::types::Priority::Sync
                    && !(is_boundary_behavior(dep.bundle_behavior)
                      || is_boundary_behavior(target_asset.bundle_behavior))
                  {
                    continue;
                  }

                  // Only async bundle roots have bundle groups.
                  let target_bundle_id = ideal_graph
                    .assets
                    .key_for(&target_asset.id)
                    .map(types::IdealBundleId::from_asset_key);
                  let is_bundle_root = target_bundle_id
                    .as_ref()
                    .is_some_and(|id| ideal_graph.get_bundle(id).is_some());

                  if !is_bundle_root {
                    continue;
                  }

                  let target_bg_key = bg_key(&default_target.name, &target_asset.id);
                  let Some(&to_bg_node_id) =
                    bundle_graph.get_node_id_by_content_key(&target_bg_key)
                  else {
                    continue;
                  };

                  let native_edge_type = NativeBundleGraphEdgeType::Bundle;
                  if seen_bundle_to_bg_edges.insert((
                    bundle_node_id,
                    to_bg_node_id,
                    native_edge_type,
                  )) {
                    bundle_graph.add_edge(&bundle_node_id, &to_bg_node_id, native_edge_type);
                  }
                }
              }
            }
          }
        }
      }
    }

    // Handle internalized bundles.
    //
    // Internalized async bundles are skipped during materialization (no bundle node and
    // no bundle group). However, the base bundle graph copied from the asset graph still
    // contains dependency -> asset `Null` edges for lazy deps that target the internalized
    // async root.
    //
    // To prevent runtime resolution from trying to load a missing async bundle, we mirror
    // the JS bundler's `internalizeAsyncDependency` behavior:
    // 1) Change dep -> asset edge type from `Null` to `References`
    // 2) Add an `InternalAsync` edge from each bundle that contains the dep -> dependency node
    {
      use std::collections::HashSet;

      // Dedup edges we add to avoid ballooning the graph.
      let mut seen: HashSet<(usize, usize, NativeBundleGraphEdgeType)> = HashSet::new();

      for internalized_bundle_id in ideal_graph.internalized_bundles.keys() {
        let root_key = internalized_bundle_id.as_asset_key();
        let root_asset_id = ideal_graph.assets.id_for(root_key);

        let Some(&root_asset_node_id) = bundle_graph.get_node_id_by_content_key(root_asset_id)
        else {
          continue;
        };

        let Some(deps) = boundary_deps_by_target_asset_id.get(root_asset_id) else {
          continue;
        };

        for dep in deps {
          if dep.priority != atlaspack_core::types::Priority::Lazy {
            continue;
          }

          let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id) else {
            continue;
          };

          // Swap dep -> asset edge: Null -> References.
          bundle_graph.remove_edge(
            &dep_bg_node_id,
            &root_asset_node_id,
            NativeBundleGraphEdgeType::Null,
          );

          if seen.insert((
            dep_bg_node_id,
            root_asset_node_id,
            NativeBundleGraphEdgeType::References,
          )) {
            bundle_graph.add_edge(
              &dep_bg_node_id,
              &root_asset_node_id,
              NativeBundleGraphEdgeType::References,
            );
          }

          // Capture bundle groups reachable from this dependency via Null edges BEFORE we
          // remove them. `_removeExternalDependency` needs this information to potentially
          // remove `bundle -> bundle_group` edges for bundles that internalized this dep.
          let mut reachable_bundle_group_node_ids: Vec<usize> = Vec::new();
          {
            let outgoing_neighbors = bundle_graph.get_outgoing_neighbors(&dep_bg_node_id);
            for neighbor in outgoing_neighbors {
              let is_bundle_group = matches!(
                bundle_graph.get_node(&neighbor),
                Some(NativeBundleGraphNode::BundleGroup { .. })
              );
              if !is_bundle_group {
                continue;
              }

              // Only consider bundle groups connected via Null edges (mirrors JS traversal).
              if bundle_graph.has_edge(&dep_bg_node_id, &neighbor, NativeBundleGraphEdgeType::Null)
              {
                reachable_bundle_group_node_ids.push(neighbor);
              }
            }
          }

          // If this dependency still points at any bundle groups via untyped/Null edges,
          // `resolveAsyncDependency` on the JS side may incorrectly treat it as an external
          // async boundary (bundle group) rather than an internalized async dependency.
          //
          // Mirror the JS bundler's `internalizeAsyncDependency` behavior by removing any
          // `dependency -> bundle_group` Null edges.
          // Note: NativeBundleGraph currently doesn't expose an "outgoing neighbors by edge type"
          // helper, so we filter by node type and remove only the `Null`-typed edge.
          for bundle_group_node_id in &reachable_bundle_group_node_ids {
            bundle_graph.remove_edge(
              &dep_bg_node_id,
              bundle_group_node_id,
              NativeBundleGraphEdgeType::Null,
            );
          }

          // Add InternalAsync edges from each bundle that contains this dep -> dep.
          //
          // This matches JS's `internalizeAsyncDependency`, which is called for every bundle
          // that contains the dependency, not just the ideal-graph's phase-8 parent bundles.
          let parent_bundle_node_ids = bundle_graph.get_incoming_neighbors_by_edge_type(
            &dep_bg_node_id,
            NativeBundleGraphEdgeType::Contains,
          );

          for parent_bundle_node_id in &parent_bundle_node_ids {
            let is_bundle = matches!(
              bundle_graph.get_node(parent_bundle_node_id),
              Some(NativeBundleGraphNode::Bundle(_))
            );

            if !is_bundle {
              continue;
            }

            if seen.insert((
              *parent_bundle_node_id,
              dep_bg_node_id,
              NativeBundleGraphEdgeType::InternalAsync,
            )) {
              bundle_graph.add_edge(
                parent_bundle_node_id,
                &dep_bg_node_id,
                NativeBundleGraphEdgeType::InternalAsync,
              );
            }
          }

          // Remove external dependency edges (JS: `_removeExternalDependency`).
          //
          // Now that this dep is internalized, we may no longer need `Bundle` edges from
          // containing bundles -> bundle groups reachable from this dependency.
          for bundle_group_node_id in reachable_bundle_group_node_ids {
            // Only remove edges for bundle groups that are connected from this dep.
            let inbound_dep_node_ids = bundle_graph.get_incoming_neighbors_by_edge_type(
              &bundle_group_node_id,
              NativeBundleGraphEdgeType::Null,
            );

            // Only consider inbound dependency nodes.
            let inbound_dep_node_ids: Vec<usize> = inbound_dep_node_ids
              .into_iter()
              .filter(|id| {
                matches!(
                  bundle_graph.get_node(id),
                  Some(NativeBundleGraphNode::Dependency(_))
                )
              })
              .collect();

            for parent_bundle_node_id in &parent_bundle_node_ids {
              // Skip non-bundles (defensive).
              if !matches!(
                bundle_graph.get_node(parent_bundle_node_id),
                Some(NativeBundleGraphNode::Bundle(_))
              ) {
                continue;
              }

              if !bundle_graph.has_edge(
                parent_bundle_node_id,
                &bundle_group_node_id,
                NativeBundleGraphEdgeType::Bundle,
              ) {
                continue;
              }

              let can_remove = inbound_dep_node_ids.iter().all(|inbound_dep_node_id| {
                let Some(NativeBundleGraphNode::Dependency(inbound_dep)) =
                  bundle_graph.get_node(inbound_dep_node_id)
                else {
                  return true;
                };

                // If there's a URL dependency inbound to the group, do not remove the edge.
                if inbound_dep.specifier_type == atlaspack_core::types::SpecifierType::Url {
                  return false;
                }

                let contained = bundle_graph.has_edge(
                  parent_bundle_node_id,
                  inbound_dep_node_id,
                  NativeBundleGraphEdgeType::Contains,
                );

                if !contained {
                  // Dependency isn't in this bundle; doesn't block removal.
                  return true;
                }

                // If the dependency is in the bundle, it must be internalized.
                bundle_graph.has_edge(
                  parent_bundle_node_id,
                  inbound_dep_node_id,
                  NativeBundleGraphEdgeType::InternalAsync,
                )
              });

              if can_remove {
                bundle_graph.remove_edge(
                  parent_bundle_node_id,
                  &bundle_group_node_id,
                  NativeBundleGraphEdgeType::Bundle,
                );
              }
            }
          }
        }
      }
    }

    Ok(())
  }
}

/// Materialize an IdealBundle into the NativeBundleGraph, returning its node id.
#[allow(clippy::too_many_arguments)]
fn materialize_ideal_bundle(
  bundle_graph: &mut NativeBundleGraph,
  _ideal_graph: &types::IdealGraph,
  ideal_bundle_id: &types::IdealBundleId,
  ideal_bundle: &types::IdealBundle,
  target: &atlaspack_core::types::Target,
  _entry_dep: &atlaspack_core::types::Dependency,
  materialized: &mut std::collections::HashMap<types::IdealBundleId, usize>,
  asset_graph: &AssetGraph,
) -> anyhow::Result<usize> {
  use atlaspack_core::hash::hash_string;
  use atlaspack_core::types::Bundle;

  if let Some(&existing) = materialized.get(ideal_bundle_id) {
    return Ok(existing);
  }

  let ideal_bundle_id_str = _ideal_graph.assets.id_for(ideal_bundle_id.as_asset_key());

  let bundle_id = hash_string(format!(
    "bundle:{}{}",
    ideal_bundle_id_str,
    target.dist_dir.display()
  ));

  let hash_reference = format!("HASH_REF_{}", &bundle_id[..16]);

  let entry_asset_id = ideal_bundle.root_asset_id.clone();
  // Determine is_splittable from the root asset's `is_bundle_splittable` property.
  // This matches the JS bundler's behavior where `isSplittable` is set from
  // `entryAsset.isBundleSplittable`.
  //
  // IMPORTANT: Shared bundles (no root asset) have `bundle.isSplittable === undefined` in the JS
  // bundle graph. `isAssetReachableFromBundle` treats a falsy `isSplittable` as "not reachable",
  // which is required so runtime helper assets (js-loader.js, bundle-url.js, cacheLoader.js)
  // are NOT considered reachable from the shared bundle and therefore are injected into it.
  let is_splittable = entry_asset_id.as_ref().and_then(|id| {
    let node_id = asset_graph.get_node_id_by_content_key(id)?;
    let asset = asset_graph.get_asset(node_id)?;
    Some(asset.is_bundle_splittable)
  });

  let bundle = Bundle {
    id: bundle_id.clone(),
    public_id: None,
    hash_reference,
    bundle_type: ideal_bundle.bundle_type.clone(),
    env: (*target.env).clone(),
    entry_asset_ids: entry_asset_id.clone().into_iter().collect(),
    main_entry_id: entry_asset_id,
    // Use the ideal-graph decision for stable naming. The builder sets
    // `needs_stable_name` correctly for every bundle type:
    //  - Entry bundles: true (is_entry == true in Phase 3)
    //  - Type-change siblings: inherited from parent (Fix 2a in css_packager)
    //  - Shared / async bundles: false (hardcoded in Phase 9)
    // Trusting the builder avoids special-casing shared vs sibling bundles here.
    needs_stable_name: Some(ideal_bundle.needs_stable_name),
    bundle_behavior: ideal_bundle.behavior,
    is_placeholder: false,
    is_splittable,
    manual_shared_bundle: None,
    name: None,
    pipeline: None,
    target: target.clone(),
  };

  let node_id = bundle_graph.add_bundle(bundle);
  materialized.insert(*ideal_bundle_id, node_id);
  Ok(node_id)
}

/// Generate a bundle group content key from a target name and asset ID.
fn bg_key(target_name: &str, asset_id: &str) -> String {
  format!("bundle_group:{}{}", target_name, asset_id)
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::types::Priority;
  use atlaspack_core::{
    asset_graph::AssetGraph,
    bundle_graph::BundleGraph,
    types::{Asset, Dependency, Environment, Target},
  };
  use pretty_assertions::assert_eq;

  use super::*;

  // ---------------------------------------------------------------------------
  // assert_graph! macro — declarative bundle shape + edge assertions
  // ---------------------------------------------------------------------------
  //
  // Usage:
  //   assert_graph!(g, {
  //     bundles: {
  //       "entry.js"     => ["entry.js", "a.js"],
  //       "async.js"     => ["async.js"],
  //       shared(vendor) => ["react.js"],
  //     },
  //     edges: {
  //       "entry.js" lazy "async.js",
  //       "entry.js" sync shared(vendor),
  //     }
  //   });
  //
  // - Named bundles use their root asset id as the key (e.g. "entry.js").
  // - `shared(label)` declares/references a shared bundle with a user-defined label.
  //   The label ties bundle declarations to edge references. Multiple shared bundles
  //   can be distinguished by giving each a unique label.
  // - Edge types: sync, lazy, parallel, conditional.
  // - Invariants (no duplication, consistent asset_to_bundle) are always checked.

  /// Internal: identifier for a bundle in macro assertions.
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  enum BundleRef {
    Named(String),
    Shared(String),
  }

  /// Internal: expected edge in macro assertions.
  #[derive(Debug, Clone)]
  struct ExpectedEdge {
    from: BundleRef,
    to: BundleRef,
    edge_type: types::IdealEdgeType,
  }

  fn format_bundle_snapshot(g: &IdealGraph) -> String {
    let mut bundle_ids: Vec<types::IdealBundleId> = g.bundles_iter().map(|(id, _)| *id).collect();
    bundle_ids.sort();

    let mut out = String::new();
    for bundle_id in bundle_ids {
      let bundle_id_str = g.assets.id_for(bundle_id.as_asset_key());
      let bundle = g.get_bundle(&bundle_id).expect("bundle must exist");
      let mut assets: Vec<&str> = bundle
        .assets
        .ones()
        .map(|idx| g.assets.id_for(types::AssetKey(idx as u32)))
        .collect();
      assets.sort();
      out.push_str(&format!("  {} => [{}]\n", bundle_id_str, assets.join(", ")));
    }

    if !g.bundle_edges.is_empty() {
      out.push_str("  edges:\n");
      for (from, to, ty) in &g.bundle_edges {
        let from_str = g.assets.id_for(from.as_asset_key());
        let to_str = g.assets.id_for(to.as_asset_key());
        out.push_str(&format!("    {} {:?} {}\n", from_str, ty, to_str));
      }
    }

    out
  }

  fn assert_graph_impl(
    g: &IdealGraph,
    expected_bundles: &[(BundleRef, Vec<&str>)],
    expected_edges: &[ExpectedEdge],
  ) {
    // 1. Always check invariants.
    assert_invariants(g);

    // 2. Check bundle contents.
    // Build actual: sort asset lists for comparison.
    let mut actual: Vec<Vec<String>> = g
      .bundles_iter()
      .map(|(_id, b)| b)
      .map(|b| {
        let mut assets: Vec<String> = b
          .assets
          .ones()
          .map(|idx| g.assets.id_for(types::AssetKey(idx as u32)).to_string())
          .collect();
        assets.sort();
        assets
      })
      .collect();
    actual.sort();

    let mut expected: Vec<Vec<String>> = expected_bundles
      .iter()
      .map(|(_, assets)| {
        let mut a: Vec<String> = assets.iter().map(|s| s.to_string()).collect();
        a.sort();
        a
      })
      .collect();
    expected.sort();

    let snapshot = format_bundle_snapshot(g);
    assert_eq!(
      expected, actual,
      "\n\nBundle mismatch.\nActual graph:\n{}",
      snapshot
    );

    // 3. Check edges.
    // Build a map from shared labels to their actual bundle ids in the graph.
    // A labeled shared bundle is matched by finding the shared bundle (@@shared:*)
    // whose assets match the expected assets declared for that label.
    let resolve_bundle_ref = |bundle_ref: &BundleRef| -> Vec<&types::IdealBundleId> {
      match bundle_ref {
        BundleRef::Named(name) => {
          let asset_key = g
            .assets
            .key_for(name)
            .unwrap_or_else(|| panic!("Edge bundle '{}' not found in AssetInterner", name));
          let id = types::IdealBundleId::from_asset_key(asset_key);
          if g.get_bundle(&id).is_some() {
            // return a stable reference for comparisons
            vec![Box::leak(Box::new(id))]
          } else {
            panic!(
              "Edge bundle '{}' not found.\nActual graph:\n{}",
              name, snapshot
            );
          }
        }
        BundleRef::Shared(label) => {
          // Find the expected assets for this label from the bundles declaration.
          let expected_assets: Vec<String> = expected_bundles
            .iter()
            .find(|(r, _)| r == bundle_ref)
            .map(|(_, assets)| {
              let mut a: Vec<String> = assets.iter().map(|s| s.to_string()).collect();
              a.sort();
              a
            })
            .unwrap_or_default();

          // Match against actual shared bundles by asset contents.
          let matches: Vec<_> = g
            .bundles_iter()
            .filter(|(id, _)| g.assets.id_for(id.as_asset_key()).starts_with("@@shared:"))
            .filter(|(_id, bundle)| {
              let mut actual_assets: Vec<String> = bundle
                .assets
                .ones()
                .map(|idx| g.assets.id_for(types::AssetKey(idx as u32)).to_string())
                .collect();
              actual_assets.sort();
              actual_assets == expected_assets
            })
            .map(|(id, _)| id)
            .collect();

          assert!(
            !matches.is_empty(),
            "Shared bundle '{}' with assets {:?} not found.\nActual graph:\n{}",
            label,
            expected_assets,
            snapshot
          );
          matches
        }
      }
    };

    for edge in expected_edges {
      let from_matches = resolve_bundle_ref(&edge.from);
      let to_matches = resolve_bundle_ref(&edge.to);

      let found = from_matches.iter().any(|from_id| {
        to_matches.iter().any(|to_id| {
          g.bundle_edges
            .iter()
            .any(|(f, t, ty)| f == *from_id && t == *to_id && *ty == edge.edge_type)
        })
      });

      assert!(
        found,
        "Expected edge {:?} {:?} {:?} not found.\nActual graph:\n{}",
        edge.from, edge.edge_type, edge.to, snapshot
      );
    }

    // 4. Verify no unexpected edges exist (exact match).
    if !expected_edges.is_empty() {
      assert_eq!(
        expected_edges.len(),
        g.bundle_edges.len(),
        "\n\nEdge count mismatch: expected {} edges, found {}.\nActual graph:\n{}",
        expected_edges.len(),
        g.bundle_edges.len(),
        snapshot
      );
    }
  }

  /// Parses an edge type keyword into an IdealEdgeType.
  macro_rules! edge_type {
    (sync) => {
      types::IdealEdgeType::Sync
    };
    (lazy) => {
      types::IdealEdgeType::Lazy
    };
    (parallel) => {
      types::IdealEdgeType::Parallel
    };
    (conditional) => {
      types::IdealEdgeType::Conditional
    };
  }

  /// Internal: accumulates bundle entries from the macro.
  macro_rules! push_bundles {
    // Base case: done.
    (@acc $bundles:ident $(,)?) => {};
    // shared(label) => [assets]
    (@acc $bundles:ident shared($label:ident) => [ $( $asset:literal ),* $(,)? ] $(, $($rest:tt)*)?) => {
      #[allow(clippy::vec_init_then_push)]
      { $bundles.push((BundleRef::Shared(stringify!($label).to_string()), vec![ $( $asset ),* ])); }
      $( push_bundles!(@acc $bundles $( $rest )*); )?
    };
    // "name" => [assets]
    (@acc $bundles:ident $name:literal => [ $( $asset:literal ),* $(,)? ] $(, $($rest:tt)*)?) => {
      #[allow(clippy::vec_init_then_push)]
      { $bundles.push((BundleRef::Named($name.to_string()), vec![ $( $asset ),* ])); }
      $( push_bundles!(@acc $bundles $( $rest )*); )?
    };
  }

  /// Internal: accumulates edge entries from the macro.
  macro_rules! push_edges {
    (@acc $edges:ident $(,)?) => {};
    // shared(label) edgetype shared(label)
    (@acc $edges:ident shared($fl:ident) $ety:ident shared($tl:ident) $(, $($rest:tt)*)?) => {
      #[allow(clippy::vec_init_then_push)]
      { $edges.push(ExpectedEdge {
        from: BundleRef::Shared(stringify!($fl).to_string()),
        to: BundleRef::Shared(stringify!($tl).to_string()),
        edge_type: edge_type!($ety),
      }); }
      $( push_edges!(@acc $edges $( $rest )*); )?
    };
    // shared(label) edgetype "name"
    (@acc $edges:ident shared($fl:ident) $ety:ident $to:literal $(, $($rest:tt)*)?) => {
      #[allow(clippy::vec_init_then_push)]
      { $edges.push(ExpectedEdge {
        from: BundleRef::Shared(stringify!($fl).to_string()),
        to: BundleRef::Named($to.to_string()),
        edge_type: edge_type!($ety),
      }); }
      $( push_edges!(@acc $edges $( $rest )*); )?
    };
    // "name" edgetype shared(label)
    (@acc $edges:ident $from:literal $ety:ident shared($tl:ident) $(, $($rest:tt)*)?) => {
      #[allow(clippy::vec_init_then_push)]
      { $edges.push(ExpectedEdge {
        from: BundleRef::Named($from.to_string()),
        to: BundleRef::Shared(stringify!($tl).to_string()),
        edge_type: edge_type!($ety),
      }); }
      $( push_edges!(@acc $edges $( $rest )*); )?
    };
    // "name" edgetype "name"
    (@acc $edges:ident $from:literal $ety:ident $to:literal $(, $($rest:tt)*)?) => {
      #[allow(clippy::vec_init_then_push)]
      { $edges.push(ExpectedEdge {
        from: BundleRef::Named($from.to_string()),
        to: BundleRef::Named($to.to_string()),
        edge_type: edge_type!($ety),
      }); }
      $( push_edges!(@acc $edges $( $rest )*); )?
    };
  }

  macro_rules! assert_graph {
    ($g:expr, {
      bundles: { $($bundles:tt)* }
      $(, edges: { $($edges:tt)* } )?
      $(,)?
    }) => {{
      #[allow(clippy::vec_init_then_push, unused_mut)]
      {
        let mut expected_bundles: Vec<(BundleRef, Vec<&str>)> = Vec::new();
        push_bundles!(@acc expected_bundles $($bundles)*);
        let mut expected_edges: Vec<ExpectedEdge> = Vec::new();
        $( push_edges!(@acc expected_edges $($edges)*); )?
        assert_graph_impl(&$g, &expected_bundles, &expected_edges);
      }
    }};
  }

  fn edge_triples(bg: &NativeBundleGraph) -> Vec<(usize, usize, NativeBundleGraphEdgeType)> {
    use petgraph::visit::{EdgeRef, IntoEdgeReferences};

    bg.graph
      .edge_references()
      .filter_map(|e| {
        let from = *bg.graph.node_weight(e.source())?;
        let to = *bg.graph.node_weight(e.target())?;
        Some((from, to, *e.weight()))
      })
      .collect()
  }

  #[test]
  fn entry_with_async_boundary() {
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[EdgeSpec::new("entry.js", "async.js", Priority::Lazy)],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "async.js" => ["async.js"],
      },
      edges: {
        "entry.js" lazy "async.js",
      },
    });
  }

  /// Builds a small asset graph for tests using a concise list of edges.
  ///
  /// Notes:
  /// - Asset ids are human-friendly file names (e.g. `entry.js`, `a.js`, `react.js`).
  /// - `entries` can contain multiple entry assets.
  /// - file types are inferred from the extension.
  #[derive(Clone, Copy, Debug)]
  struct EdgeSpec<'a> {
    from: &'a str,
    to: &'a str,
    /// Dependency specifier (used to generate a stable dependency id).
    ///
    /// This is separate from `to` because a single source asset can legitimately have
    /// multiple dependencies that resolve to the same target asset (e.g. `import('./b')`
    /// and `import bSync from './b'`). In real graphs these have distinct dependency ids.
    specifier: Option<&'a str>,
    priority: atlaspack_core::types::Priority,
    bundle_behavior: Option<atlaspack_core::types::BundleBehavior>,
  }

  impl<'a> EdgeSpec<'a> {
    fn new(from: &'a str, to: &'a str, priority: atlaspack_core::types::Priority) -> Self {
      Self {
        from,
        to,
        specifier: None,
        priority,
        bundle_behavior: None,
      }
    }

    fn specifier(mut self, specifier: &'a str) -> Self {
      self.specifier = Some(specifier);
      self
    }

    fn isolated(mut self) -> Self {
      self.bundle_behavior = Some(atlaspack_core::types::BundleBehavior::Isolated);
      self
    }

    fn inline(mut self) -> Self {
      self.bundle_behavior = Some(atlaspack_core::types::BundleBehavior::Inline);
      self
    }
  }

  fn infer_file_type(name: &str) -> atlaspack_core::types::FileType {
    let ext = std::path::Path::new(name)
      .extension()
      .and_then(|e| e.to_str())
      .unwrap_or("js");
    atlaspack_core::types::FileType::from_extension(ext)
  }

  #[derive(Clone, Debug)]
  struct AssetOptions {
    is_bundle_splittable: bool,
    bundle_behavior: Option<atlaspack_core::types::BundleBehavior>,
  }

  fn fixture_graph_with_options(
    entries: &[&str],
    edges: &[EdgeSpec<'_>],
    asset_options: &[(&str, AssetOptions)],
  ) -> AssetGraph {
    let options_map: std::collections::HashMap<&str, &AssetOptions> =
      asset_options.iter().map(|(k, v)| (*k, v)).collect();
    fixture_graph_inner(entries, edges, &options_map)
  }

  fn fixture_graph(entries: &[&str], edges: &[EdgeSpec<'_>]) -> AssetGraph {
    fixture_graph_inner(entries, edges, &std::collections::HashMap::new())
  }

  fn fixture_graph_inner(
    entries: &[&str],
    edges: &[EdgeSpec<'_>],
    asset_options: &std::collections::HashMap<&str, &AssetOptions>,
  ) -> AssetGraph {
    let mut asset_graph = AssetGraph::new();

    let mut asset_nodes: std::collections::HashMap<String, usize> =
      std::collections::HashMap::new();

    // Create entry dep(s) + entry asset(s).
    for entry in entries {
      let target = Target::default();
      let entry_dep = Dependency::entry((*entry).to_string(), target);
      let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

      let mut asset = Asset {
        id: (*entry).into(),
        file_path: (*entry).into(),
        file_type: infer_file_type(entry),
        env: Arc::new(Environment::default()),
        is_bundle_splittable: true,
        ..Asset::default()
      };
      if let Some(opts) = asset_options.get(entry) {
        asset.is_bundle_splittable = opts.is_bundle_splittable;
        asset.bundle_behavior = opts.bundle_behavior;
      }
      let entry_asset_node = asset_graph.add_asset(Arc::new(asset), false);
      asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

      asset_nodes.insert((*entry).to_string(), entry_asset_node);
    }

    // Ensure all asset nodes exist.
    for edge in edges {
      for id in [edge.from, edge.to] {
        if asset_nodes.contains_key(id) {
          continue;
        }

        let mut asset = Asset {
          id: id.into(),
          file_path: id.into(),
          file_type: infer_file_type(id),
          env: Arc::new(Environment::default()),
          is_bundle_splittable: true,
          ..Asset::default()
        };
        if let Some(opts) = asset_options.get(id) {
          asset.is_bundle_splittable = opts.is_bundle_splittable;
          asset.bundle_behavior = opts.bundle_behavior;
        }
        let node = asset_graph.add_asset(Arc::new(asset), false);
        asset_nodes.insert(id.into(), node);
      }
    }

    // Add deps.
    for edge in edges {
      let mut dep_builder = atlaspack_core::types::DependencyBuilder::default();
      dep_builder = dep_builder
        .specifier(edge.specifier.unwrap_or(edge.to).to_string())
        .specifier_type(atlaspack_core::types::SpecifierType::Esm)
        .env(Arc::new(Environment::default()))
        .priority(edge.priority)
        .source_asset_id(edge.from.into());

      if let Some(behavior) = edge.bundle_behavior {
        dep_builder = dep_builder.bundle_behavior(Some(behavior));
      }

      let dep = dep_builder.build();
      let dep_node = asset_graph.add_dependency(dep, false);
      asset_graph.add_edge(asset_nodes.get(edge.from).unwrap(), &dep_node);
      asset_graph.add_edge(&dep_node, asset_nodes.get(edge.to).unwrap());
    }

    asset_graph
  }

  fn assert_invariants(g: &IdealGraph) {
    // asset_to_bundle is consistent with bundle.assets
    // asset_to_bundle is consistent with bundle.assets
    for (asset_idx, maybe_bundle_id) in g.asset_to_bundle.iter().enumerate() {
      let Some(bundle_id) = maybe_bundle_id else {
        continue;
      };
      let asset = types::AssetKey(asset_idx as u32);
      let bundle = g.get_bundle(bundle_id).expect("bundle must exist");
      assert!(
        bundle.assets.contains(asset.0 as usize),
        "asset_to_bundle points to bundle that doesn't contain it: {:?} -> {}\n{}",
        asset,
        bundle_id.0,
        format_bundle_snapshot(g)
      );
    }

    // No asset appears in more than one splittable bundle.
    // Duplication across entry-like bundles (entries, non-splittable, isolated,
    // needs-stable-name) is expected -- they must be self-contained.
    // For simplicity, we allow duplication across any non-shared rooted bundles.
    let entry_like_bundles: std::collections::HashSet<types::IdealBundleId> = g
      .bundles_iter()
      .map(|(_id, b)| b)
      .filter(|b| b.root_asset_id.is_some())
      .map(|b| b.id)
      .collect();

    let mut seen: std::collections::HashMap<&str, types::IdealBundleId> =
      std::collections::HashMap::new();
    for (bundle_id, bundle) in g.bundles_iter() {
      for asset_idx in bundle.assets.ones() {
        let asset = g.assets.id_for(types::AssetKey(asset_idx as u32));
        if let Some(prev) = seen.insert(asset, *bundle_id) {
          // Allow duplication if either bundle is entry-like (rooted, non-shared).
          // Entry-like bundles must be self-contained, but we also allow a shared bundle
          // to contain the same asset (canonical assignment lives in the shared bundle).
          let allow_duplicate =
            entry_like_bundles.contains(&prev) || entry_like_bundles.contains(bundle_id);
          if !allow_duplicate {
            panic!(
              "asset appears in multiple bundles unexpectedly: {asset} in {} and {}\n{}",
              g.assets.id_for(prev.as_asset_key()),
              g.assets.id_for(bundle_id.as_asset_key()),
              format_bundle_snapshot(g)
            );
          }
        }
      }
    }
  }

  #[test]
  fn creates_shared_bundle_for_asset_needed_by_more_than_two_async_roots() {
    // entry -> lazy a, lazy b, lazy c; all three sync-import react.
    //
    // Shared bundles should only be created when an asset is reachable from > 2 eligible roots
    // (matching JS default `minBundles = 2`).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("a.js", "react.js", Priority::Sync),
        EdgeSpec::new("b.js", "react.js", Priority::Sync),
        EdgeSpec::new("c.js", "react.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
        shared(react) => ["react.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "entry.js" lazy "c.js",
        "a.js" sync shared(react),
        "b.js" sync shared(react),
        "c.js" sync shared(react),
      },
    });
  }

  #[test]
  fn creates_shared_bundle_for_asset_needed_by_exactly_two_async_roots() {
    // entry -> lazy a, lazy b; both a and b sync-import react
    //
    // With JS default `minBundles = 1`, this SHOULD create a shared bundle.
    // `react.js` is extracted into a shared bundle referenced by both async bundles.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "react.js", Priority::Sync),
        EdgeSpec::new("b.js", "react.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        shared(react) => ["react.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "a.js" sync shared(react),
        "b.js" sync shared(react),
      },
    });
  }

  #[test]
  fn follows_sync_edges_through_bundle_root_boundaries_for_shared_bundle_eligibility() {
    // Regression test for sync-graph reachability: sync edges should traverse THROUGH
    // bundle-root assets when computing eligible roots for shared bundles.
    //
    // Graph:
    // entry -> lazy route-1, lazy route-2
    // route-1 -> sync comp-x, lazy comp-y
    // route-2 -> sync comp-x
    // comp-x  -> sync comp-y   (sync edge to a boundary!)
    // comp-y  -> sync lib-z
    //
    // With JS default `minBundles = 1`, assets reachable from exactly two eligible roots are
    // extracted into a shared bundle.
    //
    // Note: comp-y is an async bundle root (lazy boundary) that is internalized.
    // The internalized bundle is deleted (JS `deleteBundle`).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "route-1.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "route-2.js", Priority::Lazy),
        EdgeSpec::new("route-1.js", "comp-x.js", Priority::Sync),
        EdgeSpec::new("route-1.js", "comp-y.js", Priority::Lazy).specifier("comp-y.js?lazy"),
        EdgeSpec::new("route-2.js", "comp-x.js", Priority::Sync),
        EdgeSpec::new("comp-x.js", "comp-y.js", Priority::Sync).specifier("comp-y.js?sync"),
        EdgeSpec::new("comp-y.js", "lib-z.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js"    => ["entry.js"],
        "route-1.js"  => ["route-1.js"],
        "route-2.js"  => ["route-2.js"],
        shared(shared) => ["comp-x.js", "comp-y.js", "lib-z.js"],
      },
      edges: {
        "entry.js" lazy "route-1.js",
        "entry.js" lazy "route-2.js",
        "route-1.js" sync shared(shared),
        "route-2.js" sync shared(shared),
      },
    });
  }

  #[test]
  fn boundary_created_for_parallel_and_conditional() {
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Parallel),
        EdgeSpec::new("entry.js", "b.js", Priority::Conditional),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
      },
      edges: {
        "entry.js" parallel "a.js",
        "entry.js" conditional "b.js",
      },
    });
  }

  #[test]
  fn boundary_created_for_type_change() {
    // Sync type-change deps (e.g. JS -> CSS) are NOT bundle boundaries.
    // They are placed into a type-change sibling bundle during Phase 6 placement.
    //
    // Isolated deps still create boundaries (e.g. URL/SVG-style assets).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "styles.css", Priority::Sync),
        EdgeSpec::new("entry.js", "icon.svg", Priority::Sync).isolated(),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "icon.svg" => ["icon.svg"],
        "@@typechange:entry.js:Css" => ["styles.css"],
      },
      edges: {
        "entry.js" sync "icon.svg",
        "entry.js" sync "@@typechange:entry.js:Css",
      },
    });
  }

  #[test]
  fn test_sync_type_change_from_non_root_asset() {
    // entry.js -> sync utils.js (same type, not a bundle root)
    // utils.js -> sync icon.svg (type change)
    //
    // Sync type-change deps are NOT boundaries. The SVG asset is placed into a
    // type-change sibling bundle rooted at the nearest containing bundle root.
    //
    // The bundle edge should be from the containing entry.js bundle -> type-change sibling,
    // even though the dependency originates from an unplaced non-root asset at Phase 4.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "utils.js", Priority::Sync),
        EdgeSpec::new("utils.js", "icon.svg", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "utils.js"],
        "@@typechange:entry.js:Other(\"svg\")" => ["icon.svg"],
      },
      edges: {
        "entry.js" sync "@@typechange:entry.js:Other(\"svg\")",
      },
    });
  }

  #[test]
  fn test_sync_type_change_from_deep_non_root_asset() {
    // entry.js -> a.js -> b.js -> styles.css (all sync)
    //
    // Sync type-change deps are NOT boundaries. styles.css is placed into a
    // type-change sibling bundle rooted at the containing entry bundle.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Sync),
        EdgeSpec::new("b.js", "styles.css", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js"],
        "@@typechange:entry.js:Css" => ["styles.css"],
      },
      edges: {
        "entry.js" sync "@@typechange:entry.js:Css",
      },
    });
  }

  #[test]
  fn materialization_creates_bundle_group_for_isolated_asset_reached_via_sync_dep() {
    use atlaspack_core::bundle_graph::NativeBundleGraph;
    use atlaspack_core::types::{BundleBehavior, Priority};

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.js";
    let icon = "1111111111111111.svg";

    // entry.js -> (sync) icon.svg where the *asset* icon.svg has bundleBehavior=Isolated.
    //
    // Regression: sync deps normally don't create bundle groups, but isolated assets must
    // still get a bundle group so they are discoverable from the root when traversing the
    // materialized NativeBundleGraph.
    let asset_graph = fixture_graph_with_options(
      &[entry],
      &[EdgeSpec::new(entry, icon, Priority::Sync)],
      &[(
        icon,
        AssetOptions {
          is_bundle_splittable: true,
          bundle_behavior: Some(BundleBehavior::Isolated),
        },
      )],
    );

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    let root_node_id = *bundle_graph
      .get_node_id_by_content_key("@@root")
      .expect("expected @@root node id");

    // Find the bundle group node for the isolated icon bundle.
    let icon_bg_node_id = bundle_graph
      .nodes()
      .enumerate()
      .find_map(|(id, n)| match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          entry_asset_id,
          ..
        } if entry_asset_id == icon => Some(id),
        _ => None,
      })
      .expect("expected a bundle group for isolated asset icon.svg");

    // The sync dependency node that targets icon.svg should connect to the icon bundle group.
    // Without the regression fix, this edge is missing because sync deps were skipped when
    // building `boundary_deps_by_target_asset_id`.
    let sync_dep_id = asset_graph
      .get_dependencies()
      .find(|dep| {
        if dep.is_entry {
          return false;
        }
        if dep.priority != Priority::Sync {
          return false;
        }
        if dep.source_asset_id.as_deref() != Some(entry) {
          return false;
        }

        asset_graph
          .get_node_id_by_content_key(&dep.id)
          .map(|dep_node| {
            asset_graph
              .get_outgoing_neighbors(dep_node)
              .iter()
              .any(|n| {
                asset_graph
                  .get_asset(n)
                  .is_some_and(|asset| asset.id == icon)
              })
          })
          .unwrap_or(false)
      })
      .map(|dep| dep.id.clone())
      .expect("expected a sync dependency entry -> icon");

    let sync_dep_node_id = *bundle_graph
      .get_node_id_by_content_key(&sync_dep_id)
      .expect("expected sync dependency node id");

    assert!(
      bundle_graph.has_edge(
        &sync_dep_node_id,
        &icon_bg_node_id,
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphEdgeType::Null
      ),
      "expected Null edge from sync dep -> icon bundle group"
    );

    // And the icon bundle group must be reachable from the root.
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut stack = vec![root_node_id];
    while let Some(node) = stack.pop() {
      if !seen.insert(node) {
        continue;
      }
      for next in bundle_graph.get_outgoing_neighbors(&node) {
        stack.push(next);
      }
    }

    assert!(
      seen.contains(&icon_bg_node_id),
      "expected icon bundle group to be reachable from @@root"
    );
  }

  #[test]
  fn materialization_materializes_sync_type_change_boundary_as_sibling_bundle() {
    use atlaspack_core::bundle_graph::NativeBundleGraph;
    use atlaspack_core::types::{FileType, Priority};

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.js";
    let styles = "fedcba9876543210.css";

    let asset_graph = fixture_graph(&[entry], &[EdgeSpec::new(entry, styles, Priority::Sync)]);

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    // Find the bundle node for entry.js and the CSS type-change sibling bundle.
    let bundles = bundle_graph.get_bundles();

    let entry_bundle_id = bundles
      .iter()
      .find(|b| b.main_entry_id.as_deref() == Some(entry))
      .unwrap()
      .id
      .clone();
    let entry_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&entry_bundle_id)
      .unwrap();

    let styles_bundle = bundles
      .iter()
      .find(|b| b.bundle_type == FileType::Css)
      .expect("expected a CSS type-change bundle");

    // Sync type-change sibling bundles have no main entry id.
    assert_eq!(styles_bundle.main_entry_id.as_deref(), None);

    let styles_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&styles_bundle.id)
      .unwrap();

    let entry_bg_key = bg_key(&Target::default().name, entry);
    let entry_bg_node = *bundle_graph
      .get_node_id_by_content_key(&entry_bg_key)
      .unwrap();

    let edges = edge_triples(&bundle_graph);

    // Entry bundle should reference the CSS sibling bundle.
    assert!(
      edges.contains(&(
        entry_bundle_node,
        styles_bundle_node,
        NativeBundleGraphEdgeType::References
      )),
      "expected References edge from entry bundle -> styles bundle; got edges: {:?}",
      edges
    );

    // CSS bundle should be in the same bundle group as entry (sibling bundle).
    assert!(
      edges.contains(&(
        entry_bg_node,
        styles_bundle_node,
        NativeBundleGraphEdgeType::Bundle
      )),
      "expected Bundle edge from entry bundle group -> styles bundle; got edges: {:?}",
      edges
    );

    // And the CSS bundle should contain the styles asset.
    let styles_asset_node = *bundle_graph
      .get_node_id_by_content_key(styles)
      .expect("expected styles asset node in bundle graph");

    assert!(
      edges.contains(&(
        styles_bundle_node,
        styles_asset_node,
        NativeBundleGraphEdgeType::Contains
      )),
      "expected Contains edge from styles bundle -> styles asset; got edges: {:?}",
      edges
    );

    // And styles should NOT have its own bundle group.
    let styles_bg_key = bg_key(&Target::default().name, styles);
    assert!(
      bundle_graph
        .get_node_id_by_content_key(&styles_bg_key)
        .is_none(),
      "did not expect a bundle group for styles type-change bundle"
    );
  }

  #[test]
  fn materialization_skips_bundle_edge_for_sync_dep_to_existing_async_bundle_group() {
    // This also asserts we do not connect the sync dependency node to the async bundle group.

    use atlaspack_core::bundle_graph::NativeBundleGraph;
    use atlaspack_core::types::Priority;

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.js";
    let a = "1111111111111111.js";
    let c = "2222222222222222.js";

    // entry -> lazy c (so c has its own async bundle group)
    // entry -> lazy a
    // a -> sync c (sync dep to existing async root)
    //
    // `a -> sync c` must NOT create a Bundle(3) edge to c's bundle group.
    let asset_graph = fixture_graph(
      &[entry],
      &[
        EdgeSpec::new(entry, a, Priority::Lazy),
        EdgeSpec::new(entry, c, Priority::Lazy),
        EdgeSpec::new(a, c, Priority::Sync),
      ],
    );

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    // Find the bundle nodes for entry, a, and c, plus the bundle group node for c.
    let bundles = bundle_graph.get_bundles();

    let entry_bundle_id = bundles
      .iter()
      .find(|b| b.main_entry_id.as_deref() == Some(entry))
      .unwrap()
      .id
      .clone();
    let entry_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&entry_bundle_id)
      .unwrap();

    let a_bundle_id = bundles
      .iter()
      .find(|b| b.main_entry_id.as_deref() == Some(a))
      .unwrap()
      .id
      .clone();
    let a_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&a_bundle_id)
      .unwrap();

    let c_bundle_id = bundles
      .iter()
      .find(|b| b.main_entry_id.as_deref() == Some(c))
      .unwrap()
      .id
      .clone();
    let c_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&c_bundle_id)
      .unwrap();

    let c_bg_node = bundle_graph
      .nodes()
      .enumerate()
      .find_map(|(id, n)| match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          entry_asset_id,
          ..
        } if entry_asset_id == c => Some(id),
        _ => None,
      })
      .expect("expected a bundle group for async root c");

    let edges = edge_triples(&bundle_graph);

    // entry (async parent) should have a Bundle edge to c's bundle group.
    assert!(
      edges.contains(&(
        entry_bundle_node,
        c_bg_node,
        NativeBundleGraphEdgeType::Bundle
      )),
      "expected Bundle edge from entry bundle -> c bundle group; got edges: {:?}",
      edges
    );

    // `a -> sync c` is a sync dep to an existing async bundle group. This must not
    // create a Bundle edge.
    assert!(
      !edges.contains(&(a_bundle_node, c_bg_node, NativeBundleGraphEdgeType::Bundle)),
      "did not expect Bundle edge from a bundle -> c bundle group; got edges: {:?}",
      edges
    );

    // And it must not connect the dependency node to the async bundle group.
    let a_sync_dep_id = asset_graph
      .get_dependencies()
      .find(|dep| {
        if dep.is_entry {
          return false;
        }
        if dep.priority != Priority::Sync {
          return false;
        }
        if dep.source_asset_id.as_deref() != Some(a) {
          return false;
        }

        asset_graph
          .get_node_id_by_content_key(&dep.id)
          .map(|dep_node| {
            asset_graph
              .get_outgoing_neighbors(dep_node)
              .iter()
              .any(|n| asset_graph.get_asset(n).is_some_and(|asset| asset.id == c))
          })
          .unwrap_or(false)
      })
      .map(|dep| dep.id.clone())
      .expect("expected a sync dependency a -> c");

    let a_sync_dep_node = *bundle_graph
      .get_node_id_by_content_key(&a_sync_dep_id)
      .expect("expected sync dependency node in bundle graph");

    assert!(
      !edges.contains(&(a_sync_dep_node, c_bg_node, NativeBundleGraphEdgeType::Null)),
      "did not expect Null edge from sync dependency -> c bundle group; got edges: {:?}",
      edges
    );

    // But it should create a References edge (bundle-to-bundle relationship).
    assert!(
      edges.contains(&(
        a_bundle_node,
        c_bundle_node,
        NativeBundleGraphEdgeType::References
      )),
      "expected References edge from a bundle -> c bundle; got edges: {:?}",
      edges
    );
  }

  #[test]
  fn materialization_does_not_add_sync_referenced_async_bundle_as_sibling_in_source_group() {
    use atlaspack_core::bundle_graph::NativeBundleGraph;
    use atlaspack_core::types::Priority;

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.js";
    let a = "1111111111111111.js";
    let b = "3333333333333333.js";
    let c = "2222222222222222.js";

    // entry -> lazy a, lazy b, lazy c
    // a -> sync c, b -> sync c
    let asset_graph = fixture_graph(
      &[entry],
      &[
        EdgeSpec::new(entry, a, Priority::Lazy),
        EdgeSpec::new(entry, b, Priority::Lazy),
        EdgeSpec::new(entry, c, Priority::Lazy),
        EdgeSpec::new(a, c, Priority::Sync),
        EdgeSpec::new(b, c, Priority::Sync),
      ],
    );

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    let bundles = bundle_graph.get_bundles();

    let a_bundle_id = bundles
      .iter()
      .find(|bundle| bundle.main_entry_id.as_deref() == Some(a))
      .unwrap()
      .id
      .clone();
    let a_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&a_bundle_id)
      .unwrap();

    let b_bundle_id = bundles
      .iter()
      .find(|bundle| bundle.main_entry_id.as_deref() == Some(b))
      .unwrap()
      .id
      .clone();
    let b_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&b_bundle_id)
      .unwrap();

    let c_bundle_id = bundles
      .iter()
      .find(|bundle| bundle.main_entry_id.as_deref() == Some(c))
      .unwrap()
      .id
      .clone();
    let c_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&c_bundle_id)
      .unwrap();

    let a_bg_node = bundle_graph
      .nodes()
      .enumerate()
      .find_map(|(id, n)| match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          entry_asset_id,
          ..
        } if entry_asset_id == a => Some(id),
        _ => None,
      })
      .expect("expected a bundle group for async root a");

    let b_bg_node = bundle_graph
      .nodes()
      .enumerate()
      .find_map(|(id, n)| match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          entry_asset_id,
          ..
        } if entry_asset_id == b => Some(id),
        _ => None,
      })
      .expect("expected a bundle group for async root b");

    let edges = edge_triples(&bundle_graph);

    // a bundle group should contain only a bundle.
    assert!(
      edges.contains(&(a_bg_node, a_bundle_node, NativeBundleGraphEdgeType::Bundle)),
      "expected Bundle edge from a bundle group -> a bundle; got edges: {:?}",
      edges
    );
    assert!(
      !edges.contains(&(a_bg_node, c_bundle_node, NativeBundleGraphEdgeType::Bundle)),
      "did not expect c bundle to be sibling in a bundle group; got edges: {:?}",
      edges
    );

    // b bundle group should contain only b bundle.
    assert!(
      edges.contains(&(b_bg_node, b_bundle_node, NativeBundleGraphEdgeType::Bundle)),
      "expected Bundle edge from b bundle group -> b bundle; got edges: {:?}",
      edges
    );
    assert!(
      !edges.contains(&(b_bg_node, c_bundle_node, NativeBundleGraphEdgeType::Bundle)),
      "did not expect c bundle to be sibling in b bundle group; got edges: {:?}",
      edges
    );

    // `entry -> lazy c` is internalized (c is sync-reachable from all async parent bundles),
    // so it must not keep a dependency -> bundle_group Null edge.
    let entry_lazy_c_dep_id = asset_graph
      .get_dependencies()
      .find(|dep| {
        if dep.is_entry {
          return false;
        }
        if dep.priority != Priority::Lazy {
          return false;
        }
        if dep.source_asset_id.as_deref() != Some(entry) {
          return false;
        }

        asset_graph
          .get_node_id_by_content_key(&dep.id)
          .map(|dep_node| {
            asset_graph
              .get_outgoing_neighbors(dep_node)
              .iter()
              .any(|n| asset_graph.get_asset(n).is_some_and(|asset| asset.id == c))
          })
          .unwrap_or(false)
      })
      .map(|dep| dep.id.clone())
      .expect("expected a lazy dependency entry -> c");

    let entry_lazy_c_dep_node = *bundle_graph
      .get_node_id_by_content_key(&entry_lazy_c_dep_id)
      .expect("expected lazy dependency node in bundle graph");

    // Find c's bundle group node.
    let c_bg_node = bundle_graph
      .nodes()
      .enumerate()
      .find_map(|(id, n)| match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          entry_asset_id,
          ..
        } if entry_asset_id == c => Some(id),
        _ => None,
      })
      .expect("expected a bundle group for async root c");

    let entry_bundle_id = bundles
      .iter()
      .find(|bundle| bundle.main_entry_id.as_deref() == Some(entry))
      .unwrap()
      .id
      .clone();
    let entry_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&entry_bundle_id)
      .unwrap();

    // Only assert internalization invariants if the dependency was actually internalized.
    if edges.contains(&(
      entry_bundle_node,
      entry_lazy_c_dep_node,
      NativeBundleGraphEdgeType::InternalAsync,
    )) {
      assert!(
        !edges.contains(&(
          entry_lazy_c_dep_node,
          c_bg_node,
          NativeBundleGraphEdgeType::Null
        )),
        "did not expect Null edge from internalized lazy dep -> c bundle group; got edges: {:?}",
        edges
      );

      // `_removeExternalDependency` invariant: once the dep is internalized, the containing
      // bundle should no longer have a `Bundle` edge to the async bundle group.
      assert!(
        !edges.contains(&(
          entry_bundle_node,
          c_bg_node,
          NativeBundleGraphEdgeType::Bundle
        )),
        "did not expect Bundle edge from entry bundle -> c bundle group after internalization; got edges: {:?}",
        edges
      );
    }
  }

  #[test]
  fn multiple_entries_create_multiple_root_bundles() {
    let asset_graph = fixture_graph(
      &["entry-a.js", "entry-b.js"],
      &[
        EdgeSpec::new("entry-a.js", "a.js", Priority::Sync),
        EdgeSpec::new("entry-b.js", "b.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry-a.js" => ["entry-a.js", "a.js"],
        "entry-b.js" => ["entry-b.js", "b.js"],
      },
    });
  }

  #[test]
  fn shared_bundle_groups_multiple_assets_for_same_root_set() {
    // Both a and b import react + react-dom.
    // With JS default `minBundles = 1`, this should extract a shared bundle for both assets.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "react.js", Priority::Sync),
        EdgeSpec::new("b.js", "react.js", Priority::Sync),
        EdgeSpec::new("a.js", "react-dom.js", Priority::Sync),
        EdgeSpec::new("b.js", "react-dom.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        shared(vendor) => ["react-dom.js", "react.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "a.js" sync shared(vendor),
        "b.js" sync shared(vendor),
      },
    });
  }

  #[test]
  fn sync_only_stays_in_single_bundle() {
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js"],
      },
    });
  }

  #[test]
  fn dominator_diamond_has_single_owner() {
    // entry -> a -> (b,c) -> d — all sync, single bundle
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Sync),
        EdgeSpec::new("a.js", "c.js", Priority::Sync),
        EdgeSpec::new("b.js", "d.js", Priority::Sync),
        EdgeSpec::new("c.js", "d.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js", "c.js", "d.js"],
      },
    });
  }

  #[test]
  fn two_different_root_sets_create_two_shared_bundles() {
    // entry -> lazy a,b,c
    // a,b share x ; b,c share y
    //
    // With JS default `minBundles = 1`, both x and y should be extracted into shared bundles
    // (one per root set).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("a.js", "x.js", Priority::Sync),
        EdgeSpec::new("b.js", "x.js", Priority::Sync),
        EdgeSpec::new("b.js", "y.js", Priority::Sync),
        EdgeSpec::new("c.js", "y.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
        shared(x)  => ["x.js"],
        shared(y)  => ["y.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "entry.js" lazy "c.js",
        "a.js" sync shared(x),
        "b.js" sync shared(x),
        "b.js" sync shared(y),
        "c.js" sync shared(y),
      },
    });
  }

  #[test]
  fn shared_bundle_edges_are_deduped() {
    // Verify bundle edges are deduped when creating a shared bundle.
    // Use 3 roots so a shared bundle is actually created under `minBundles = 2`.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("a.js", "react.js", Priority::Sync),
        EdgeSpec::new("b.js", "react.js", Priority::Sync),
        EdgeSpec::new("c.js", "react.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
        shared(react) => ["react.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "entry.js" lazy "c.js",
        "a.js" sync shared(react),
        "b.js" sync shared(react),
        "c.js" sync shared(react),
      },
    });
  }

  #[test]
  fn availability_intersection_does_not_duplicate_disjoint_assets() {
    // entry -> lazy a, lazy b
    // a -> sync x, b -> sync y
    // a -> lazy c, b -> lazy c
    //
    // c has two parents with disjoint sync trees.
    // x and y should NOT appear in c's bundle (they aren't available from both parents).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "x.js", Priority::Sync),
        EdgeSpec::new("b.js", "y.js", Priority::Sync),
        EdgeSpec::new("a.js", "c.js", Priority::Lazy),
        EdgeSpec::new("b.js", "c.js", Priority::Lazy),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js", "x.js"],
        "b.js"     => ["b.js", "y.js"],
        "c.js"     => ["c.js"],
      },
    });
  }

  #[test]
  fn multi_entry_shared_asset_is_duplicated_into_all_entries() {
    // entry-a and entry-b both sync-import shared.js.
    // Each entry must independently contain shared.js because either entry
    // could be loaded on its own. This must be duplication, not a shared bundle.
    let asset_graph = fixture_graph(
      &["entry-a.js", "entry-b.js"],
      &[
        EdgeSpec::new("entry-a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("entry-b.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry-a.js" => ["entry-a.js", "shared.js"],
        "entry-b.js" => ["entry-b.js", "shared.js"],
      },
    });
  }

  #[test]
  fn non_splittable_bundles_receive_duplicated_assets_like_entries() {
    // entry -> lazy a, lazy b
    // a -> sync react.js, b -> sync react.js
    // a is non-splittable (isBundleSplittable = false)
    //
    // The JS algorithm treats non-splittable bundles like entries: assets are
    // duplicated into them. Since a is non-splittable, react.js should be
    // placed in a AND in a shared bundle for b (or just in b if only one
    // non-entry root remains after filtering).
    let asset_graph = fixture_graph_with_options(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "react.js", Priority::Sync),
        EdgeSpec::new("b.js", "react.js", Priority::Sync),
      ],
      &[(
        "a.js",
        AssetOptions {
          is_bundle_splittable: false,
          bundle_behavior: None,
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // react.js must be in a.js's bundle (duplicated like an entry).
    // b.js is the only remaining splittable root, so react.js goes directly into b.js.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js", "react.js"],
        "b.js"     => ["b.js", "react.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
      },
    });
  }

  #[test]
  fn internalize_async_bundle_when_root_already_available() {
    // entry -> sync a -> lazy b
    // entry -> sync b
    //
    // b.js is both sync-imported by entry and async-imported by a.
    // b.js's async bundle is internalized and deleted (JS `deleteBundle`).
    // b.js is placed into entry so it can be processed like any other sync asset.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js"],
      },
    });
  }

  #[test]
  fn internalizes_async_bundle_when_root_sync_available_from_ancestor_and_parent() {
    // Integration regression:
    // index.js --sync--> a.js
    // a.js    --sync--> b.js
    // a.js    --lazy--> b.js (separate dependency id)
    //
    // Expected: b.js ends up in the entry bundle (sync-available from ancestor), and the
    // async bundle for b.js is deleted.
    let asset_graph = fixture_graph(
      &["index.js"],
      &[
        EdgeSpec::new("index.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Sync).specifier("b.js?sync"),
        EdgeSpec::new("a.js", "b.js", Priority::Lazy).specifier("b.js?lazy"),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "index.js" => ["index.js", "a.js", "b.js"],
      },
    });
  }

  #[test]
  fn internalizes_lazy_bundle_when_root_is_sync_available_from_ancestor() {
    // This fixture internalizes b.js's async bundle, but leaves a lazy dependency in the
    // asset graph pointing at b.js.
    //
    // Materialization must rewrite that lazy dep edge to `References` and add an
    // `InternalAsync` edge from the parent bundle -> dep node so runtimes don't try
    // to resolve b.js via a missing async bundle.

    // Mirrors integration fixture:
    // entry -> sync a
    // a -> lazy b
    // a -> sync b
    //
    // b.js is a bundle boundary (lazy), but also sync-imported by a which is in the entry.
    // b.js's async bundle is internalized and deleted. b.js is placed into entry.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Lazy).specifier("b.js?lazy"),
        EdgeSpec::new("a.js", "b.js", Priority::Sync).specifier("b.js?sync"),
      ],
    );

    // Sanity: verify there is at least one sync dep edge a.js -> b.js.
    let a_node = *asset_graph
      .get_node_id_by_content_key("a.js")
      .expect("missing a.js node");
    let mut found_sync_b = false;
    for dep_node in asset_graph.get_outgoing_neighbors(&a_node) {
      let Some(dep) = asset_graph.get_dependency(&dep_node) else {
        continue;
      };
      if dep.priority != Priority::Sync {
        continue;
      }
      for target in asset_graph.get_outgoing_neighbors(&dep_node) {
        let Some(asset) = asset_graph.get_asset(&target) else {
          continue;
        };
        if asset.id == "b.js" {
          found_sync_b = true;
          break;
        }
      }
    }
    assert!(
      found_sync_b,
      "expected sync dep a.js -> b.js in fixture graph"
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js"],
      },
    });
  }

  #[test]
  fn internalize_async_bundle_removes_bundle_and_places_root_in_parent() {
    // (Regression) internalization should delete the async bundle so its root doesn't act as a hard boundary
    // during entry bundle Contains traversal.
    // entry -> lazy route-1
    // route-1 -> sync comp-x, lazy comp-y
    // comp-x -> sync comp-y (sync edge to a boundary)
    // comp-y -> sync lib-z
    //
    // With internalization, comp-y is no longer treated as a bundle root for placement.
    // The async bundle for comp-y is internalized and deleted, and comp-y/lib-z are placed
    // into the parent (route-1) bundle.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "route-1.js", Priority::Lazy),
        EdgeSpec::new("route-1.js", "comp-x.js", Priority::Sync),
        EdgeSpec::new("route-1.js", "comp-y.js", Priority::Lazy).specifier("comp-y.js?lazy"),
        EdgeSpec::new("comp-x.js", "comp-y.js", Priority::Sync).specifier("comp-y.js?sync"),
        EdgeSpec::new("comp-y.js", "lib-z.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "route-1.js" => ["comp-x.js", "comp-y.js", "lib-z.js", "route-1.js"],
      },
      edges: {
        "entry.js" lazy "route-1.js",
      }
    });
  }

  #[test]
  fn do_not_internalize_when_root_not_available_from_all_parents() {
    // entry -> lazy a, lazy b
    // a -> sync c
    // b -> sync c
    //
    // c.js is an async boundary from both a and b, but it's NOT available
    // from the entry bundle. The async bundle for c should NOT be internalized.
    //
    // With JS default `minBundles = 1`, c.js should be extracted into a shared bundle referenced by both a and b.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "c.js", Priority::Sync),
        EdgeSpec::new("b.js", "c.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js" => ["a.js"],
        "b.js" => ["b.js"],
        shared(c) => ["c.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "a.js" sync shared(c),
        "b.js" sync shared(c),
      },
    });
  }

  #[test]
  fn reuse_bundle_when_async_root_is_subgraph_of_another() {
    // entry -> lazy a -> sync x
    // entry -> lazy b -> sync x
    //          b -> lazy a
    //
    // x is reachable from both a and b. But b also lazy-loads a, so a's bundle
    // (containing x) is available when b loads. No shared bundle should be created.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "x.js", Priority::Sync),
        EdgeSpec::new("b.js", "x.js", Priority::Sync),
        EdgeSpec::new("b.js", "a.js", Priority::Lazy),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        shared(x)  => ["x.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "b.js" lazy "a.js",
        "a.js" sync shared(x),
        "b.js" sync shared(x),
      },
    });
  }

  #[test]
  fn reuse_existing_bundle_instead_of_creating_shared() {
    // entry -> lazy a, lazy b, lazy c
    // a -> sync c, b -> sync c
    //
    // c.js is a bundle root (lazy from entry) AND sync-reachable from a and b.
    // Instead of creating a shared bundle for c.js, we should reuse c.js's
    // existing bundle -- a and b just need sync edges to it.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("a.js", "c.js", Priority::Sync),
        EdgeSpec::new("b.js", "c.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "entry.js" lazy "c.js",
        "a.js" sync "c.js",
        "b.js" sync "c.js",
      },
    });
  }

  #[test]
  fn reuse_async_bundle_esmodule_helpers_not_in_async_bundle() {
    // index.js -> lazy a.js, lazy b.js, lazy c.js
    // a.js -> sync c.js
    // b.js -> sync c.js
    // All four files -> sync helpers.js (simulating esmodule-helpers)
    let asset_graph = fixture_graph(
      &["index.js"],
      &[
        EdgeSpec::new("index.js", "a.js", Priority::Lazy),
        EdgeSpec::new("index.js", "b.js", Priority::Lazy),
        EdgeSpec::new("index.js", "c.js", Priority::Lazy),
        EdgeSpec::new("a.js", "c.js", Priority::Sync),
        EdgeSpec::new("b.js", "c.js", Priority::Sync),
        // All ESM files import helpers
        EdgeSpec::new("index.js", "helpers.js", Priority::Sync),
        EdgeSpec::new("a.js", "helpers.js", Priority::Sync),
        EdgeSpec::new("b.js", "helpers.js", Priority::Sync),
        EdgeSpec::new("c.js", "helpers.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();
    // helpers.js should only be in the entry bundle, NOT in c.js bundle
    let c_id = types::IdealBundleId::from_asset_key(g.assets.key_for("c.js").unwrap());
    let c_bundle = g.get_bundle(&c_id).expect("c.js bundle should exist");
    assert!(
      !g.bundle_has_asset(&c_id, "helpers.js"),
      "helpers.js should NOT be in c.js bundle. c.js bundle assets: {:?}",
      c_bundle.assets
    );

    // helpers.js SHOULD be in the entry bundle
    let entry_id = types::IdealBundleId::from_asset_key(g.assets.key_for("index.js").unwrap());
    let entry_bundle = g.get_bundle(&entry_id).expect("entry bundle should exist");
    assert!(
      g.bundle_has_asset(&entry_id, "helpers.js"),
      "helpers.js should be in entry bundle. Entry bundle assets: {:?}",
      entry_bundle.assets
    );
  }

  #[test]
  fn parallel_sibling_availability_prevents_shared_bundle() {
    // entry -> parallel a -> sync shared
    // entry -> parallel b -> sync shared
    //
    // a and b load in parallel. Since a is ordered before b (deterministic),
    // b should see a's assets as available. shared.js should be in a's bundle
    // and available to b via parallel sibling availability -- no shared bundle needed.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Parallel),
        EdgeSpec::new("entry.js", "b.js", Priority::Parallel),
        EdgeSpec::new("a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("b.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js", "shared.js"],
        "b.js"     => ["b.js"],
      },
      edges: {
        "entry.js" parallel "b.js",
        "entry.js" parallel "a.js",
      },
    });
  }

  #[test]
  fn shared_extraction_is_suppressed_when_asset_already_available() {
    // vendor is sync-imported by entry AND both async roots.
    // Since it's already in the entry bundle, no shared bundle is needed.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "vendor.js", Priority::Sync),
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "vendor.js", Priority::Sync),
        EdgeSpec::new("b.js", "vendor.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "vendor.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
      },
    });
  }

  #[test]
  fn deep_async_roots_shared_asset_is_handled_via_availability_or_shared_bundle() {
    // Fixture from availability divergence investigation:
    // A (entry) --sync--> B
    // B --lazy--> C --sync--> D --lazy--> E --sync--> F
    // B --lazy--> G --sync--> F
    //
    // F is reachable from both E and G. At availability time, each bundle initially contains
    // only its root asset(s), and availability unions in sync-reachable assets (stopping at
    // bundle boundaries). This test ensures the Rust implementation matches that behavior.
    //
    // Expected: F is extracted into a shared bundle referenced by both E and G.
    let asset_graph = fixture_graph(
      &["a.js"],
      &[
        EdgeSpec::new("a.js", "b.js", Priority::Sync),
        EdgeSpec::new("b.js", "c.js", Priority::Lazy),
        EdgeSpec::new("c.js", "d.js", Priority::Sync),
        EdgeSpec::new("d.js", "e.js", Priority::Lazy),
        EdgeSpec::new("e.js", "f.js", Priority::Sync),
        EdgeSpec::new("b.js", "g.js", Priority::Lazy),
        EdgeSpec::new("g.js", "f.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "a.js" => ["a.js", "b.js"],
        "c.js" => ["c.js", "d.js"],
        "e.js" => ["e.js"],
        "g.js" => ["g.js"],
        shared(f) => ["f.js"],
      },
      edges: {
        "a.js" lazy "c.js",
        "a.js" lazy "g.js",
        "c.js" lazy "e.js",
        "e.js" sync shared(f),
        "g.js" sync shared(f),
      },
    });
  }

  #[test]
  fn root_assets_are_never_moved_into_shared_bundles() {
    // a and b are lazy roots AND sync-imported by c.
    // They must stay in their own bundles, never extracted to shared.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("c.js", "a.js", Priority::Sync),
        EdgeSpec::new("c.js", "b.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
      },
    });
  }

  #[test]
  fn isolated_dependency_prevents_shared_extraction() {
    // a -> (sync isolated) react, b -> (sync) react
    // The isolated edge creates a boundary for react, making it a named bundle.
    // Without isolated, react would be in a shared bundle instead.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "react.js", Priority::Sync).isolated(),
        EdgeSpec::new("b.js", "react.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // react.js is a named bundle (boundary from isolated dep), NOT a shared bundle.
    // The edges show react.js is only reachable from b.js, not from a.js.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "react.js" => ["react.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "a.js" sync "react.js",
        "b.js" sync "react.js",
      },
    });
  }

  #[test]
  fn availability_includes_sync_reachable_assets_from_parallel_siblings() {
    // entry -> lazy page
    // page -> parallel sidebar
    // page -> sync shared_lib
    // sidebar -> sync shared_lib
    //
    // Without sync-reachable availability: shared_lib is reachable from both
    // page and sidebar, which would create a shared bundle.
    //
    // With proper availability: sidebar is loaded in parallel with page.
    // page already contains shared_lib (sync-reachable), so sidebar's
    // availability includes shared_lib via its parallel sibling (page).
    // Therefore shared_lib should NOT be in a shared bundle - it should
    // be placed directly in page's bundle, and sidebar should know it's available.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "page.js", Priority::Lazy),
        EdgeSpec::new("page.js", "sidebar.js", Priority::Parallel),
        EdgeSpec::new("page.js", "shared_lib.js", Priority::Sync),
        EdgeSpec::new("sidebar.js", "shared_lib.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // shared_lib.js should be in page.js's bundle (sync-reachable from page).
    // sidebar.js should know shared_lib is available via parallel loading.
    // NO shared bundle should be created for shared_lib.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "page.js" => ["page.js", "shared_lib.js"],
        "sidebar.js" => ["sidebar.js"],
      },
      edges: {
        "entry.js" lazy "page.js",
        "page.js" parallel "sidebar.js",
      },
    });
  }

  #[test]
  fn availability_includes_sync_reachable_assets_from_ancestor_bundles() {
    // entry -> lazy page
    // page -> lazy dialog
    // page -> sync util
    // dialog -> sync util
    //
    // util.js is sync-reachable from page.js (placed in page's bundle).
    // dialog.js is lazy-loaded from page.js, so page's bundle is an ancestor.
    // Therefore util.js should be available to dialog via ancestor_assets.
    // NO shared bundle should be created.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "page.js", Priority::Lazy),
        EdgeSpec::new("page.js", "dialog.js", Priority::Lazy),
        EdgeSpec::new("page.js", "util.js", Priority::Sync),
        EdgeSpec::new("dialog.js", "util.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // util.js placed in page.js bundle. dialog knows it's available from ancestor.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "page.js" => ["page.js", "util.js"],
        "dialog.js" => ["dialog.js"],
      },
      edges: {
        "entry.js" lazy "page.js",
        "page.js" lazy "dialog.js",
      },
    });
  }

  #[test]
  fn deep_async_chain_shared_absorbed_into_first_root() {
    // entry -> lazy a; a -> sync shared, lazy b; b -> lazy c; c -> sync shared
    //
    // shared.js is sync-reachable from both a and c (two async roots).
    // Availability propagates through the async chain: entry -> a -> b -> c.
    // Since a's bundle contains shared.js, it becomes available to b and then c
    // via ancestor_assets. This means c does NOT need its own copy of shared.js,
    // reducing the eligible roots to just [a]. shared.js is therefore absorbed
    // directly into a's bundle rather than being extracted into a shared bundle.
    //
    // This matches the JS bundler's behavior, verified by the integration test
    // "Deep async chain: Entry → async A → async B → async C, with a module shared by A and C".
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Lazy),
        EdgeSpec::new("b.js", "c.js", Priority::Lazy),
        EdgeSpec::new("c.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js", "shared.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "a.js" lazy "b.js",
        "b.js" lazy "c.js",
      },
    });
  }

  #[test]
  fn many_async_roots_with_overlapping_shared_subsets() {
    // 4 async roots with pairwise-shared modules.
    // With JS default `minBundles = 1`, each pairwise-shared module should be extracted into a
    // shared bundle (one per root set).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "d.js", Priority::Lazy),
        EdgeSpec::new("a.js", "shared_ab.js", Priority::Sync),
        EdgeSpec::new("a.js", "shared_ac.js", Priority::Sync),
        EdgeSpec::new("b.js", "shared_ab.js", Priority::Sync),
        EdgeSpec::new("b.js", "shared_bd.js", Priority::Sync),
        EdgeSpec::new("c.js", "shared_ac.js", Priority::Sync),
        EdgeSpec::new("c.js", "shared_cd.js", Priority::Sync),
        EdgeSpec::new("d.js", "shared_bd.js", Priority::Sync),
        EdgeSpec::new("d.js", "shared_cd.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
        "d.js"     => ["d.js"],
        shared(ab) => ["shared_ab.js"],
        shared(ac) => ["shared_ac.js"],
        shared(bd) => ["shared_bd.js"],
        shared(cd) => ["shared_cd.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "entry.js" lazy "c.js",
        "entry.js" lazy "d.js",
        "a.js" sync shared(ab),
        "a.js" sync shared(ac),
        "b.js" sync shared(ab),
        "b.js" sync shared(bd),
        "c.js" sync shared(ac),
        "c.js" sync shared(cd),
        "d.js" sync shared(bd),
        "d.js" sync shared(cd),
      },
    });
  }

  #[test]
  fn subgraph_reuse_prevents_shared_bundle_when_root_reachable() {
    // Scenario:
    //   entry -> lazy a
    //   entry -> lazy b
    //   b -> sync a
    //   a -> sync shared
    //   b -> sync shared
    //
    // `shared.js` is reachable from both `a` and `b`, but `b` can sync-reach `a`.
    // JS Insert Or Share "subgraph absorption" removes `b` from the eligible set,
    // so `shared.js` is placed into `a`'s bundle and no shared bundle is created.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("b.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("b.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js" => ["a.js", "shared.js"],
        "b.js" => ["b.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "b.js" sync "a.js",
      },
    });
  }

  #[test]
  fn lazy_dep_inside_shared_bundle_creates_bundle_group_edge() {
    // Regression test: shared bundles contain lazy deps whose bundle groups must be
    // reachable from the root. Without the fix in the non-entry Contains traversal,
    // the `shared -> locale` bundle_group edge was missing, making locale's bundle
    // unreachable via traverseBundles and causing a nullthrows crash during naming.

    use atlaspack_core::bundle_graph::NativeBundleGraph;

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.js";
    let a = "1111111111111111.js";
    let b = "2222222222222222.js";
    let shared = "3333333333333333.js";
    let locale = "4444444444444444.js";

    let asset_graph = fixture_graph(
      &[entry],
      &[
        EdgeSpec::new(entry, a, Priority::Lazy),
        EdgeSpec::new(entry, b, Priority::Lazy),
        EdgeSpec::new(entry, "5555555555555555.js", Priority::Lazy),
        EdgeSpec::new(a, shared, Priority::Sync),
        EdgeSpec::new(b, shared, Priority::Sync),
        EdgeSpec::new("5555555555555555.js", shared, Priority::Sync),
        EdgeSpec::new(shared, locale, Priority::Lazy),
      ],
    );

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    // Find the shared bundle node (shared bundles have no main entry id).
    let shared_bundle_id = bundle_graph
      .get_bundles()
      .into_iter()
      .find(|bundle| bundle.main_entry_id.is_none())
      .expect("expected a shared bundle")
      .id
      .clone();

    let shared_bundle_node = *bundle_graph
      .get_node_id_by_content_key(&shared_bundle_id)
      .expect("expected shared bundle node id");

    // Find the bundle group node for the async locale bundle.
    let locale_bg_node = bundle_graph
      .nodes()
      .enumerate()
      .find_map(|(id, n)| match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          entry_asset_id,
          ..
        } if entry_asset_id == locale => Some(id),
        _ => None,
      })
      .expect("expected a bundle group for async root locale");

    let edges = edge_triples(&bundle_graph);

    // Critical assertion: shared bundle must have a Bundle edge to locale's bundle group.
    assert!(
      edges.contains(&(
        shared_bundle_node,
        locale_bg_node,
        NativeBundleGraphEdgeType::Bundle
      )),
      "expected Bundle edge from shared bundle -> locale bundle group; got edges: {:?}",
      edges
    );
  }

  #[test]
  fn bundle_group_coload_propagates_availability_to_lazy_children() {
    // entry -> lazy page.js -> parallel page.css -> sync shared.js
    //                page.js -> lazy dialog.js -> sync shared.js
    //
    // page.js and page.css are in the same bundle group (parallel edge).
    // page.css sync-reaches shared.js.
    // dialog.js is a lazy child of page.js and also sync-reaches shared.js.
    //
    // With bundle group co-load: when computing availability for page.js,
    // we union in page.css's reachable assets (since page.css is in the same
    // bundle group). shared.js becomes part of page.js's `available`, which
    // propagates to dialog.js as ancestor_assets. dialog.js then sees shared.js
    // as available and doesn't need a shared bundle.
    //
    // shared.js should be placed in page.css's bundle (first splittable root
    // via Phase 7 co-load placement).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "page.js", Priority::Lazy),
        EdgeSpec::new("page.js", "page.css", Priority::Parallel),
        EdgeSpec::new("page.js", "dialog.js", Priority::Lazy),
        EdgeSpec::new("page.css", "shared.js", Priority::Sync),
        EdgeSpec::new("dialog.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // The key assertion: no shared bundle is created for shared.js.
    // shared.js ends up in dialog.js's bundle (first splittable root after
    // availability filtering removes page.css).
    assert_graph!(g, {
      bundles: {
        "entry.js"  => ["entry.js"],
        "page.js"   => ["page.js"],
        "page.css"  => ["page.css"],
        "dialog.js" => ["dialog.js", "shared.js"],
      },
      edges: {
        "entry.js" lazy "page.js",
        "page.js" parallel "page.css",
        "page.js" lazy "dialog.js",
      },
    });
  }

  #[test]
  fn isolated_entry_still_discovers_lazy_children_in_bundle_root_graph() {
    // Regression test: entry assets with bundleBehavior=Isolated must still
    // discover their lazy children during bundleRootGraph BFS. Without the
    // fix, the BFS skips the entry entirely, producing 0 outgoing edges and
    // breaking availability propagation.
    let asset_graph = fixture_graph_with_options(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("b.js", "shared.js", Priority::Sync),
      ],
      &[(
        "entry.js",
        AssetOptions {
          is_bundle_splittable: true,
          bundle_behavior: Some(atlaspack_core::types::BundleBehavior::Isolated),
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // The key assertion: shared.js should be in a shared bundle (not duplicated
    // or missing), proving that the entry's lazy children were properly discovered
    // in the bundleRootGraph and availability was propagated.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js" => ["a.js"],
        "b.js" => ["b.js"],
        shared(shared) => ["shared.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "a.js" sync shared(shared),
        "b.js" sync shared(shared),
      },
    });
  }

  #[test]
  fn bundlebehavior_asset_blocks_sync_reachability_propagation() {
    // a (entry) -> sync b (asset bundleBehavior=Isolated) -> sync c
    // a (entry) -> lazy d -> sync c
    //
    // Reachability propagation should stop UPSTREAM bits at b (b has bundleBehavior),
    // meaning c should NOT be considered sync-reachable from a through b.
    // However, b's OWN root bit still propagates to c, so c IS reachable from b.
    //
    // As a result, c is reachable from both b (entry-like/isolated) and d (splittable).
    // c gets duplicated into b's bundle (entry-like always gets assets) and d's bundle.
    let asset_graph = fixture_graph_with_options(
      &["a.js"],
      &[
        EdgeSpec::new("a.js", "b.js", Priority::Sync),
        EdgeSpec::new("b.js", "c.js", Priority::Sync),
        EdgeSpec::new("a.js", "d.js", Priority::Lazy),
        EdgeSpec::new("d.js", "c.js", Priority::Sync),
      ],
      &[(
        "b.js",
        AssetOptions {
          is_bundle_splittable: true,
          bundle_behavior: Some(atlaspack_core::types::BundleBehavior::Isolated),
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        // Bundle roots are also duplicated into entry-like bundles.
        "a.js" => ["a.js", "b.js"],
        "b.js" => ["b.js", "c.js"],
        "d.js" => ["c.js", "d.js"],
      },
      edges: {
        "a.js" sync "b.js",
        "a.js" lazy "d.js",
      },
    });
  }

  #[test]
  fn entry_like_roots_always_get_assets_regardless_of_availability() {
    // entry.js --sync--> a.js --sync--> shared.js
    // entry.js --lazy--> lazy.js --sync--> shared.js
    //
    // Both entry.js and lazy.js reach shared.js.
    // Even if lazy.js sees shared.js as available via ancestors, entry.js is entry-like and must
    // ALWAYS contain shared.js (matches JS addAssetToBundleRoot semantics for entries).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("entry.js", "lazy.js", Priority::Lazy),
        EdgeSpec::new("lazy.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "shared.js"],
        // shared.js is already available from the ancestor entry bundle, so it should not be
        // duplicated into the async bundle.
        "lazy.js"  => ["lazy.js"],
      },
      edges: {
        "entry.js" lazy "lazy.js",
      },
    });
  }

  #[test]
  fn available_everywhere_assets_placed_via_coload_fallback() {
    // entry.js --lazy--> a.js --sync--> shared.js
    // entry.js --lazy--> b.js --sync--> shared.js
    // entry.js --lazy--> c.js --sync--> shared.js
    //
    // In cases where availability filtering marks `shared.js` as available from all reachable roots,
    // the asset must still be placed somewhere (co-load fallback) and not be left unplaced.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "c.js", Priority::Lazy),
        EdgeSpec::new("a.js", "shared.js", Priority::Sync),
        EdgeSpec::new("b.js", "shared.js", Priority::Sync),
        EdgeSpec::new("c.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    let shared_key = g
      .assets
      .key_for("shared.js")
      .expect("expected shared.js in AssetInterner");

    // Key assertion: shared.js must be assigned to some bundle (either a root bundle or a shared bundle).
    assert!(
      g.asset_bundle(&shared_key).is_some(),
      "shared.js was left unplaced (asset_bundle == None). Actual graph:\n{}",
      format_bundle_snapshot(&g)
    );
  }

  #[test]
  fn available_entry_asset_still_reachable_from_async() {
    // entry.js sync-imports shared.js and also lazy-loads async.js.
    // async.js sync-imports shared.js too.
    //
    // Since shared.js is already in the entry bundle, it should be available to async.js via
    // ancestor_assets and must NOT be duplicated into async.js.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "shared.js", Priority::Sync),
        EdgeSpec::new("entry.js", "async.js", Priority::Lazy),
        EdgeSpec::new("async.js", "shared.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "shared.js"],
        "async.js" => ["async.js"],
      },
      edges: {
        "entry.js" lazy "async.js",
      },
    });
  }

  #[test]
  fn bundle_behavior_asset_does_not_inflate_reachable_roots_across_lazy_boundary() {
    // Regression test for reachability calculation:
    //
    // entry --lazy--> route-a --sync--> helper
    // entry --lazy--> route-b --sync--> helper
    // route-a --lazy--> isolated (asset bundle_behavior = Isolated)
    // isolated --sync--> helper
    //
    // `isolated.js` must be treated as a separate bundle root that does NOT get added to the
    // reachable root set of upstream assets.
    //
    // The bundler *will* extract `helper.js` into a shared bundle for (route-a, route-b).
    // The regression we care about is that the shared bundle's root set must *not* also include
    // `isolated.js` (i.e. it must be shared only between route-a and route-b).
    let asset_graph = fixture_graph_with_options(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "route-a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "route-b.js", Priority::Lazy),
        EdgeSpec::new("route-a.js", "helper.js", Priority::Sync),
        EdgeSpec::new("route-b.js", "helper.js", Priority::Sync),
        EdgeSpec::new("route-a.js", "isolated.js", Priority::Lazy),
        EdgeSpec::new("isolated.js", "helper.js", Priority::Sync),
      ],
      &[(
        "isolated.js",
        AssetOptions {
          is_bundle_splittable: true,
          bundle_behavior: Some(atlaspack_core::types::BundleBehavior::Isolated),
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js"   => ["entry.js"],
        "route-a.js" => ["route-a.js"],
        "route-b.js" => ["route-b.js"],
        "isolated.js" => ["helper.js", "isolated.js"],
        // NOTE: the shared bundle label is arbitrary, but the bundle's identity and edges should
        // reflect that helper is shared *only* between route-a and route-b.
        shared(helper) => ["helper.js"],
      },
      edges: {
        "entry.js" lazy "route-a.js",
        "entry.js" lazy "route-b.js",
        "route-a.js" lazy "isolated.js",
        "route-a.js" sync shared(helper),
        "route-b.js" sync shared(helper),
      },
    });
  }

  #[test]
  fn reuses_existing_bundle_when_shared_asset_is_itself_a_lazy_root() {
    // Regression test for subgraph reuse pass 1:
    //
    // entry --lazy--> a --sync--> x
    // entry --lazy--> b --sync--> x
    // entry --lazy--> x        (x is itself a bundle root)
    //
    // `x.js` should end up in its own bundle (because it's a lazy root) and that bundle should be
    // reused when referenced from a/b, rather than duplicating x or incorrectly filtering all
    // eligible roots.
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "x.js", Priority::Lazy),
        EdgeSpec::new("a.js", "x.js", Priority::Sync),
        EdgeSpec::new("b.js", "x.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js" => ["a.js"],
        "b.js" => ["b.js"],
        "x.js" => ["x.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "entry.js" lazy "x.js",
        "a.js" sync "x.js",
        "b.js" sync "x.js",
      },
    });
  }

  #[test]
  fn entry_like_bundle_preserves_assets_when_shared_bundle_extracted() {
    // Regression test for Phase 9 shared extraction:
    //
    // When a shared bundle is extracted, assets that originated from an entry-like bundle
    // (e.g. a non-splittable bundle) must be *preserved* in that entry-like bundle as well.
    //
    // Without the fix, extracting `helper.js` into a shared bundle removed it from the
    // entry-like `non-split.js` bundle (because `move_asset_to_bundle` detached it).
    // With the fix, `helper.js` remains in `non-split.js` and is also present in the shared bundle.
    //
    // Graph:
    // entry.js --lazy--> non-split.js (non-splittable)
    // non-split.js --sync--> helper.js
    // entry.js --lazy--> route-a.js --sync--> helper.js
    // entry.js --lazy--> route-b.js --sync--> helper.js
    let asset_graph = fixture_graph_with_options(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "non-split.js", Priority::Lazy),
        EdgeSpec::new("non-split.js", "helper.js", Priority::Sync),
        EdgeSpec::new("entry.js", "route-a.js", Priority::Lazy),
        EdgeSpec::new("route-a.js", "helper.js", Priority::Sync),
        EdgeSpec::new("entry.js", "route-b.js", Priority::Lazy),
        EdgeSpec::new("route-b.js", "helper.js", Priority::Sync),
      ],
      &[(
        "non-split.js",
        AssetOptions {
          is_bundle_splittable: false,
          bundle_behavior: None,
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // Key assertions:
    // - `helper.js` is duplicated into the entry-like `non-split.js` bundle
    // - `helper.js` is also extracted into a shared bundle
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "non-split.js" => ["helper.js", "non-split.js"],
        "route-a.js" => ["route-a.js"],
        "route-b.js" => ["route-b.js"],
        shared(helper) => ["helper.js"],
      },
    });
  }

  #[test]
  fn bundle_root_duplicated_into_entry_like_bundle() {
    // Regression test for Phase 8 parent bundle roots:
    //
    // If an asset is a bundle root (async boundary) AND is also sync-imported by an entry-like
    // bundle (e.g. a non-splittable bundle), the bundle root must be duplicated into the
    // entry-like bundle.
    //
    // Without the fix, Phase 8 skipped duplicating bundle roots into entry-like bundles.
    // With the fix, `lazy-target.js` ends up in BOTH `non-split.js`'s bundle and its own bundle.
    //
    // Graph:
    // entry.js --lazy--> non-split.js (non-splittable)
    // non-split.js --sync--> lazy-target.js
    // entry.js --lazy--> lazy-target.js  (bundle root)
    // lazy-target.js --sync--> child.js
    let asset_graph = fixture_graph_with_options(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "non-split.js", Priority::Lazy),
        EdgeSpec::new("non-split.js", "lazy-target.js", Priority::Sync),
        EdgeSpec::new("entry.js", "lazy-target.js", Priority::Lazy),
        EdgeSpec::new("lazy-target.js", "child.js", Priority::Sync),
      ],
      &[(
        "non-split.js",
        AssetOptions {
          is_bundle_splittable: false,
          bundle_behavior: None,
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // Key assertions:
    // - `lazy-target.js` has its own bundle (it's a lazy root)
    // - `lazy-target.js` is also duplicated into the entry-like `non-split.js` bundle
    // - `child.js` is in both bundles since it's sync-reachable from lazy-target
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "non-split.js" => ["child.js", "lazy-target.js", "non-split.js"],
        "lazy-target.js" => ["child.js", "lazy-target.js"],
      },
      edges: {
        "entry.js" lazy "non-split.js",
        "entry.js" lazy "lazy-target.js",
        "non-split.js" sync "lazy-target.js",
      },
    });
  }

  #[test]
  fn inline_bundle_behavior_creates_boundary_not_shared_bundle() {
    // entry.html sync-imports both page.js and inline-script.js, and page.js also
    // sync-imports inline-script.js.
    //
    // The inline dependency must create a bundle boundary (similar to Isolated):
    // inline-script.js gets its own bundle and must NOT be extracted into a shared bundle.
    let asset_graph = fixture_graph(
      &["entry.html"],
      &[
        EdgeSpec::new("entry.html", "inline-script.js", Priority::Sync).inline(),
        EdgeSpec::new("entry.html", "page.js", Priority::Sync),
        EdgeSpec::new("page.js", "inline-script.js", Priority::Sync).inline(),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.html" => ["entry.html"],
        // entry.html -> page.js is a sync type-change (Html -> Js), so page.js is placed into a
        // type-change sibling bundle rooted at entry.html.
        "@@typechange:entry.html:Js" => ["page.js"],
        "inline-script.js" => ["inline-script.js"],
      },
      edges: {
        "entry.html" sync "inline-script.js",
        "entry.html" sync "@@typechange:entry.html:Js",
      },
    });
  }

  #[test]
  fn bundle_root_with_type_change_does_not_create_typechange_sibling() {
    // Regression test for `resolve_type_change_target`:
    //
    // We need a bundle root that would *also* be reachable via the sync graph from an entry,
    // such that placement tries to assign it under the entry bundle and hits a type change.
    //
    // Fixture:
    // - entry.js --lazy--> async.css        (makes async.css a *bundle root*)
    // - entry.js --sync--> lib.js
    // - lib.js   --sync--> async.css        (sync path makes the root reachable from entry)
    //
    // Without the guard in `resolve_type_change_target`, placement would create an extra
    // empty-ish type-change sibling bundle:
    //   @@typechange:entry.js:Css
    //
    // With the guard, no such sibling bundle should exist; async.css should remain its own bundle.
    let asset_graph = fixture_graph_with_options(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "async.css", Priority::Lazy),
        EdgeSpec::new("entry.js", "lib.js", Priority::Sync),
        EdgeSpec::new("lib.js", "async.css", Priority::Sync).specifier("async-sync"),
      ],
      &[(
        "async.css",
        AssetOptions {
          is_bundle_splittable: true,
          // Prevent internalization so `async.css` remains a bundle root,
          // while still being sync-reachable from the entry.
          bundle_behavior: Some(atlaspack_core::types::BundleBehavior::Inline),
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats, _report) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // async.css must remain a real bundle root bundle.
    let async_bundle_id = types::IdealBundleId::from_asset_key(
      g.assets
        .key_for("async.css")
        .expect("expected async.css in AssetInterner"),
    );
    let async_bundle = g
      .get_bundle(&async_bundle_id)
      .expect("expected an async.css bundle");
    assert!(
      g.bundle_has_asset(&async_bundle_id, "async.css"),
      "expected async.css bundle to contain async.css; got {:?}",
      async_bundle.assets
    );

    // Critical assertion: no type-change sibling rooted at entry.js should exist.
    assert!(
      g.assets.key_for("@@typechange:entry.js:Css").is_none(),
      "did not expect @@typechange:entry.js:Css bundle to exist"
    );
  }

  #[test]
  fn inline_js_dep_is_materializer_boundary() {
    use atlaspack_core::bundle_graph::NativeBundleGraph;
    use atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphEdgeType;

    // Verify that BundleBehavior::Inline deps are treated as boundaries during
    // entry bundle materialization: the inline asset and its children must NOT
    // appear in the entry bundle's Contains edges.
    //
    // Note: this is defense-in-depth — the "don't cross into other bundle roots"
    // guard in the entry DFS also prevents traversal. This test ensures the Inline
    // boundary check works correctly even if the bundle-root guard changes.

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.js";
    let inline_root = "fedcba9876543210.js";
    let inline_child = "1111111111111111.js";

    let asset_graph = fixture_graph(
      &[entry],
      &[
        // entry.js --sync,inline--> inline_root.js
        EdgeSpec::new(entry, inline_root, Priority::Sync).inline(),
        // inline_root.js --sync--> inline_child.js
        EdgeSpec::new(inline_root, inline_child, Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    let edges = edge_triples(&bundle_graph);

    // Find the entry bundle node (main_entry_id == entry.js).
    let entry_bundle_id = bundle_graph
      .get_bundles()
      .iter()
      .find(|b| b.main_entry_id.as_deref() == Some(entry))
      .expect("expected an entry bundle")
      .id
      .clone();
    let entry_bundle_node_id = *bundle_graph
      .get_node_id_by_content_key(&entry_bundle_id)
      .expect("expected entry bundle node id");

    let entry_node_id = *bundle_graph
      .get_node_id_by_content_key(entry)
      .expect("expected entry asset node id");
    let inline_root_node_id = *bundle_graph
      .get_node_id_by_content_key(inline_root)
      .expect("expected inline root asset node id");
    let inline_child_node_id = *bundle_graph
      .get_node_id_by_content_key(inline_child)
      .expect("expected inline child asset node id");

    // Entry bundle should contain entry.js.
    assert_eq!(
      edges.contains(&(
        entry_bundle_node_id,
        entry_node_id,
        NativeBundleGraphEdgeType::Contains
      )),
      true,
      "expected entry bundle to contain its entry asset; got edges: {:?}",
      edges
    );

    // Critical assertions: entry bundle must NOT traverse into inline root (or its child)
    // during Contains DFS.
    assert_eq!(
      edges.contains(&(
        entry_bundle_node_id,
        inline_root_node_id,
        NativeBundleGraphEdgeType::Contains
      )),
      false,
      "did not expect entry bundle to contain inline root asset; got edges: {:?}",
      edges
    );

    assert_eq!(
      edges.contains(&(
        entry_bundle_node_id,
        inline_child_node_id,
        NativeBundleGraphEdgeType::Contains
      )),
      false,
      "did not expect entry bundle to contain child of inline asset; got edges: {:?}",
      edges
    );

    // And inline_root.js must be contained by its own separate bundle.
    let inline_bundle_id = bundle_graph
      .get_bundles()
      .iter()
      .find(|b| b.main_entry_id.as_deref() == Some(inline_root))
      .expect("expected an inline JS bundle")
      .id
      .clone();
    let inline_bundle_node_id = *bundle_graph
      .get_node_id_by_content_key(&inline_bundle_id)
      .expect("expected inline bundle node id");

    assert_eq!(
      edges.contains(&(
        inline_bundle_node_id,
        inline_root_node_id,
        NativeBundleGraphEdgeType::Contains
      )),
      true,
      "expected inline bundle to contain its inline root asset; got edges: {:?}",
      edges
    );
  }

  #[test]
  fn inline_asset_does_not_get_bundle_group() {
    use atlaspack_core::bundle_graph::NativeBundleGraph;
    use atlaspack_core::types::Priority;

    // Regression test: inline CSS assets (from HTML <style> tags) are sync deps with
    // BundleBehavior::Inline, but they must NOT be treated as async boundary roots.
    // Otherwise they incorrectly get bundle groups and can create empty bundle groups.

    // Use hex-like ids so NativeBundleGraph public id generation won't panic.
    let entry = "0123456789abcdef.html";
    let inline_css = "fedcba9876543210.css";

    let asset_graph = fixture_graph_with_options(
      &[entry],
      &[EdgeSpec::new(entry, inline_css, Priority::Sync).inline()],
      &[(
        inline_css,
        AssetOptions {
          bundle_behavior: Some(atlaspack_core::types::BundleBehavior::Inline),
          is_bundle_splittable: true,
        },
      )],
    );

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    let has_inline_css_bundle_group = bundle_graph.nodes().any(|node| match node {
      NativeBundleGraphNode::BundleGroup { entry_asset_id, .. } => entry_asset_id == inline_css,
      _ => false,
    });

    assert_eq!(
      has_inline_css_bundle_group, false,
      "did not expect a bundle group for inline CSS asset {inline_css}"
    );
  }
}
