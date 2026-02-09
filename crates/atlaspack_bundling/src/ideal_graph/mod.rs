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
pub mod types;

use anyhow::Context;
use atlaspack_core::{
  asset_graph::AssetGraph,
  bundle_graph::{NativeBundleGraph, native_bundle_graph::NativeBundleGraphEdgeType},
};

use crate::Bundler;

use self::{
  builder::IdealGraphBuilder,
  types::{IdealGraph, IdealGraphBuildOptions, IdealGraphBuildStats},
};

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
  pub fn build_ideal_graph(
    &self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<(IdealGraph, IdealGraphBuildStats)> {
    IdealGraphBuilder::new(self.options.clone())
      .build(asset_graph)
      .context("building IdealGraph via IdealGraphBuilder")
  }
}

impl Bundler for IdealGraphBundler {
  fn bundle(
    &self,
    asset_graph: &AssetGraph,
    bundle_graph: &mut NativeBundleGraph,
  ) -> anyhow::Result<()> {
    let (ideal_graph, _stats) = self.build_ideal_graph(asset_graph)?;

    let root_node_id = *bundle_graph
      .get_node_id_by_content_key("@@root")
      .context("missing @@root node in NativeBundleGraph")?;

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
    use std::collections::HashMap;

    // Track which ideal bundles have been materialized into the NativeBundleGraph.
    let mut materialized_bundle_nodes: HashMap<String, usize> = HashMap::new();

    for (entry_dep, entry_asset_id) in &entries {
      let target: Target = entry_dep.target.as_deref().cloned().unwrap_or_default();

      // Find the ideal bundle that contains this entry asset.
      let ideal_bundle_id = ideal_graph
        .asset_to_bundle
        .get(entry_asset_id)
        .context("entry asset not assigned to any ideal bundle")?;

      let ideal_bundle = ideal_graph
        .bundles
        .get(ideal_bundle_id)
        .context("ideal bundle missing")?;

      // Create the bundle group.
      let bundle_group_id = format!("bundle_group:{}{}", target.name, entry_asset_id);
      let bundle_group_node_id =
        bundle_graph.add_bundle_group(bundle_group_id, target.clone(), entry_asset_id.clone());

      // Materialize this ideal bundle into the NativeBundleGraph if not already done.
      let bundle_node_id = materialize_ideal_bundle(
        bundle_graph,
        &ideal_graph,
        ideal_bundle_id,
        ideal_bundle,
        &target,
        entry_dep,
        &mut materialized_bundle_nodes,
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
    // and create bundle groups for them.
    let default_target = entries
      .first()
      .and_then(|(dep, _)| dep.target.as_deref().cloned())
      .unwrap_or_default();

    let default_entry_dep = entries.first().map(|(dep, _)| dep).unwrap();

    for (ideal_bundle_id, ideal_bundle) in &ideal_graph.bundles {
      if materialized_bundle_nodes.contains_key(&ideal_bundle_id.0) {
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
      )?;

      // Create a bundle group for non-entry bundles (async/shared).
      if let Some(root_asset_id) = &ideal_bundle.root_asset_id {
        let bundle_group_id = format!("bundle_group:{}{}", default_target.name, root_asset_id);
        let bundle_group_node_id = bundle_graph.add_bundle_group(
          bundle_group_id,
          default_target.clone(),
          root_asset_id.clone(),
        );

        // Find the dependency that leads to this bundle root.
        // We look for dependencies targeting this asset from the asset graph.
        let mut found_dep = false;
        for dep in asset_graph.get_dependencies() {
          if dep.is_entry {
            continue;
          }
          let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dep.id) else {
            continue;
          };
          for neighbor in asset_graph.get_outgoing_neighbors(dep_node_id) {
            if let Some(target_asset) = asset_graph.get_asset(&neighbor) {
              if target_asset.id == *root_asset_id {
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
          }
        }

        bundle_graph.add_edge(
          &root_node_id,
          &bundle_group_node_id,
          NativeBundleGraphEdgeType::Bundle,
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

        if let Some(&root_asset_node_id) = bundle_graph.get_node_id_by_content_key(root_asset_id) {
          bundle_graph.add_edge(
            &bundle_node_id,
            &root_asset_node_id,
            NativeBundleGraphEdgeType::Null,
          );
          if found_dep {
            // Add References edge from dep to asset for non-entry bundles too.
            for dep in asset_graph.get_dependencies() {
              if dep.is_entry {
                continue;
              }
              let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dep.id) else {
                continue;
              };
              for neighbor in asset_graph.get_outgoing_neighbors(dep_node_id) {
                if let Some(target_asset) = asset_graph.get_asset(&neighbor) {
                  if target_asset.id == *root_asset_id {
                    if let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id)
                    {
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
      }
    }

    // Add Contains edges by traversing the asset graph from each bundle's root asset,
    // following sync deps and stopping at bundle boundaries. This matches the behavior
    // of V2's addAssetGraphToBundle which duplicates shared sync assets into each
    // entry bundle rather than using zero-duplication placement.
    use std::collections::HashSet;

    for (ideal_bundle_id, ideal_bundle) in &ideal_graph.bundles {
      let Some(&bundle_node_id) = materialized_bundle_nodes.get(&ideal_bundle_id.0) else {
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

            // For non-sync deps (lazy/async), add a Bundle edge from this bundle
            // to the target bundle's bundle group. This is how the JS runtime's
            // getChildBundles discovers async bundles, which triggers manifest
            // registration code (including bundle-url.js).
            if dep.priority != atlaspack_core::types::Priority::Sync {
              for target_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
                if let Some(target_asset) = asset_graph.get_asset(&target_node_id) {
                  if let Some(target_bundle_id) = ideal_graph.asset_to_bundle.get(&target_asset.id)
                  {
                    if target_bundle_id != ideal_bundle_id {
                      if let Some(target_bundle) = ideal_graph.bundles.get(target_bundle_id) {
                        if let Some(root_asset_id) = &target_bundle.root_asset_id {
                          let bg_key =
                            format!("bundle_group:{}{}", default_target.name, root_asset_id);
                          if let Some(&bg_node_id) =
                            bundle_graph.get_node_id_by_content_key(&bg_key)
                          {
                            bundle_graph.add_edge(
                              &bundle_node_id,
                              &bg_node_id,
                              NativeBundleGraphEdgeType::Bundle,
                            );
                          }
                        }
                      }
                    }
                  }
                }
              }
              continue;
            }

            if dep.bundle_behavior == Some(atlaspack_core::types::BundleBehavior::Isolated)
              || dep.bundle_behavior == Some(atlaspack_core::types::BundleBehavior::InlineIsolated)
            {
              continue;
            }

            // Follow deps to target assets.
            for target_node_id in asset_graph.get_outgoing_neighbors(&dep_node_id) {
              if let Some(target_asset) = asset_graph.get_asset(&target_node_id) {
                // Don't cross into other bundle roots.
                let is_other_bundle_root = ideal_graph
                  .bundles
                  .contains_key(&types::IdealBundleId(target_asset.id.clone()))
                  && target_asset.id != ideal_bundle_id.0;

                if is_other_bundle_root {
                  continue;
                }

                stack.push(target_asset.id.clone());
              }
            }
          }
        }
      } else {
        // Non-entry bundles (async, shared): use the ideal graph's asset set directly.
        // This respects zero-duplication placement - assets available from ancestors
        // won't be duplicated into child bundles.
        for asset_id in &ideal_bundle.assets {
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
        for asset_id in &ideal_bundle.assets {
          if let Some(ag_node_id) = asset_graph.get_node_id_by_content_key(asset_id) {
            for dep_node_id in asset_graph.get_outgoing_neighbors(ag_node_id) {
              if let Some(dep) = asset_graph.get_dependency(&dep_node_id) {
                if let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id) {
                  bundle_graph.add_edge(
                    &bundle_node_id,
                    &dep_bg_node_id,
                    NativeBundleGraphEdgeType::Contains,
                  );
                }
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
fn materialize_ideal_bundle(
  bundle_graph: &mut NativeBundleGraph,
  _ideal_graph: &types::IdealGraph,
  ideal_bundle_id: &types::IdealBundleId,
  ideal_bundle: &types::IdealBundle,
  target: &atlaspack_core::types::Target,
  entry_dep: &atlaspack_core::types::Dependency,
  materialized: &mut std::collections::HashMap<String, usize>,
) -> anyhow::Result<usize> {
  use atlaspack_core::hash::hash_string;
  use atlaspack_core::types::Bundle;

  if let Some(&existing) = materialized.get(&ideal_bundle_id.0) {
    return Ok(existing);
  }

  let bundle_id = hash_string(format!(
    "bundle:{}{}",
    ideal_bundle_id.0,
    target.dist_dir.display()
  ));

  let entry_asset_id = ideal_bundle.root_asset_id.clone();

  let bundle = Bundle {
    id: bundle_id.clone(),
    public_id: None,
    hash_reference: bundle_id.clone(),
    bundle_type: ideal_bundle.bundle_type.clone(),
    env: (*target.env).clone(),
    entry_asset_ids: entry_asset_id.clone().into_iter().collect(),
    main_entry_id: entry_asset_id,
    needs_stable_name: Some(entry_dep.needs_stable_name),
    bundle_behavior: ideal_bundle.behavior,
    is_splittable: Some(false),
    manual_shared_bundle: None,
    name: None,
    pipeline: None,
    target: target.clone(),
  };

  let node_id = bundle_graph.add_bundle(bundle);
  materialized.insert(ideal_bundle_id.0.clone(), node_id);
  Ok(node_id)
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::types::Priority;
  use atlaspack_core::{
    asset_graph::AssetGraph,
    types::{Asset, Dependency, Environment, FileType, Target},
  };
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn ideal_graph_bundler_can_build_graph() {
    let mut asset_graph = AssetGraph::new();

    let target = Target::default();
    let entry_dep = Dependency::entry("entry.js".to_string(), target);
    let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

    let entry_asset = Arc::new(Asset {
      id: "entry.js".into(),
      file_path: "entry.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let entry_asset_node = asset_graph.add_asset(entry_asset, false);
    asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

    // Add a lazy dep creating an async boundary.
    let lazy_dep = atlaspack_core::types::DependencyBuilder::default()
      .specifier("./async.js".to_string())
      .specifier_type(atlaspack_core::types::SpecifierType::Esm)
      .env(Arc::new(Environment::default()))
      .priority(atlaspack_core::types::Priority::Lazy)
      .source_asset_id("entry.js".into())
      .build();
    let lazy_dep_node = asset_graph.add_dependency(lazy_dep, false);
    asset_graph.add_edge(&entry_asset_node, &lazy_dep_node);

    let async_asset = Arc::new(Asset {
      id: "async.js".into(),
      file_path: "async.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let async_asset_node = asset_graph.add_asset(async_asset, false);
    asset_graph.add_edge(&lazy_dep_node, &async_asset_node);

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_eq!(stats.assets, 2);
    assert_eq!(stats.dependencies, 2);

    // Both entry and boundary roots become bundles.
    assert_eq!(g.bundles.len(), 2);

    // Debug decision log should exist and contain boundary + placement decisions.
    let debug = g.debug.as_ref().expect("debug info should be present");
    assert!(!debug.decisions.is_empty());
    assert!(
      debug
        .decisions
        .decisions
        .iter()
        .any(|d| { matches!(d.kind, types::DecisionKind::BoundaryCreated { .. }) })
    );
    assert!(debug.decisions.decisions.iter().any(|d| {
      matches!(
        d.kind,
        types::DecisionKind::AssetAssignedToBundle { .. }
          | types::DecisionKind::BundleRootCreated { .. }
      )
    }));

    // Decisions are sequential.
    for (i, d) in debug.decisions.decisions.iter().enumerate() {
      assert_eq!(d.seq, i as u64);
    }

    // We should have a lazy bundle edge from entry bundle to the async bundle.
    assert!(g.bundle_edges.iter().any(|(from, to, ty)| {
      from.0 == "entry.js" && to.0 == "async.js" && matches!(ty, types::IdealEdgeType::Lazy)
    }));

    // NOTE: We intentionally do not call `NativeBundleGraph::from_asset_graph` here.
    // That path has stricter invariants about asset ids/public ids that aren't relevant
    // for unit testing the ideal graph pipeline.

    // Avoid unused warnings.
    let _ = (entry_asset_node, async_asset_node);
  }

  fn format_bundle_assets(g: &IdealGraph) -> String {
    let mut bundle_ids: Vec<&str> = g.bundles.keys().map(|b| b.0.as_str()).collect();
    bundle_ids.sort();

    let mut out = String::new();
    for bundle_id in bundle_ids {
      let bundle = &g.bundles[&types::IdealBundleId(bundle_id.to_string())];
      let mut assets: Vec<&str> = bundle.assets.iter().map(|s| s.as_str()).collect();
      assets.sort();
      out.push_str(bundle_id);
      out.push_str(" -> [");
      out.push_str(&assets.join(", "));
      out.push_str("]\n");
    }
    out
  }

  fn assert_bundles(g: &IdealGraph, expected_bundle_assets: &[&[&str]]) {
    // Build actual bundle asset sets (ignore bundle ids).
    let mut actual: Vec<Vec<String>> = g
      .bundles
      .values()
      .map(|b| {
        let mut assets: Vec<String> = b.assets.iter().cloned().collect();
        assets.sort();
        assets
      })
      .collect();
    actual.sort();

    let mut expected: Vec<Vec<String>> = expected_bundle_assets
      .iter()
      .map(|assets| {
        let mut a = assets.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        a.sort();
        a
      })
      .collect();
    expected.sort();

    let actual_snapshot = format_bundle_assets(g);
    assert_eq!(
      expected, actual,
      "bundle snapshot (actual):\n{}",
      actual_snapshot
    );
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
    priority: atlaspack_core::types::Priority,
    bundle_behavior: Option<atlaspack_core::types::BundleBehavior>,
  }

  impl<'a> EdgeSpec<'a> {
    fn new(from: &'a str, to: &'a str, priority: atlaspack_core::types::Priority) -> Self {
      Self {
        from,
        to,
        priority,
        bundle_behavior: None,
      }
    }

    fn isolated(mut self) -> Self {
      self.bundle_behavior = Some(atlaspack_core::types::BundleBehavior::Isolated);
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

  fn fixture_graph(entries: &[&str], edges: &[EdgeSpec<'_>]) -> AssetGraph {
    let mut asset_graph = AssetGraph::new();

    let mut asset_nodes: std::collections::HashMap<String, usize> =
      std::collections::HashMap::new();

    // Create entry dep(s) + entry asset(s).
    for entry in entries {
      let target = Target::default();
      let entry_dep = Dependency::entry((*entry).to_string(), target);
      let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

      let entry_asset = Arc::new(Asset {
        id: (*entry).into(),
        file_path: (*entry).into(),
        file_type: infer_file_type(entry),
        env: Arc::new(Environment::default()),
        ..Asset::default()
      });
      let entry_asset_node = asset_graph.add_asset(entry_asset, false);
      asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

      asset_nodes.insert((*entry).to_string(), entry_asset_node);
    }

    // Ensure all asset nodes exist.
    for edge in edges {
      for id in [edge.from, edge.to] {
        if asset_nodes.contains_key(id) {
          continue;
        }

        let asset = Arc::new(Asset {
          id: id.into(),
          file_path: id.into(),
          file_type: infer_file_type(id),
          env: Arc::new(Environment::default()),
          ..Asset::default()
        });
        let node = asset_graph.add_asset(asset, false);
        asset_nodes.insert(id.into(), node);
      }
    }

    // Add deps.
    for edge in edges {
      let mut dep_builder = atlaspack_core::types::DependencyBuilder::default();
      dep_builder = dep_builder
        .specifier(edge.to.to_string())
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
    for (asset, bundle_id) in &g.asset_to_bundle {
      let bundle = g.bundles.get(bundle_id).expect("bundle must exist");
      assert!(
        bundle.assets.contains(asset),
        "asset_to_bundle points to bundle that doesn't contain it: {asset} -> {}\n{}",
        bundle_id.0,
        format_bundle_assets(g)
      );
    }

    // no asset appears in more than one bundle
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (bundle_id, bundle) in &g.bundles {
      for asset in &bundle.assets {
        if let Some(prev) = seen.insert(asset, &bundle_id.0) {
          panic!(
            "asset appears in multiple bundles: {asset} in {prev} and {}\n{}",
            bundle_id.0,
            format_bundle_assets(g)
          );
        }
      }
    }
  }

  #[test]
  fn creates_shared_bundle_for_asset_needed_by_multiple_async_roots() {
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);
    assert_bundles(&g, &[&["entry.js"], &["a.js"], &["b.js"], &["react.js"]]);

    // Still has async edges and shared edges.
    assert!(g.bundle_edges.iter().any(|(a, b, t)| {
      a.0 == "entry.js" && b.0 == "a.js" && matches!(t, types::IdealEdgeType::Lazy)
    }));
    assert!(
      g.bundle_edges
        .iter()
        .any(|(a, b, _)| a.0 == "entry.js" && b.0.starts_with("@@shared:"))
    );
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    // entry bundle + two boundary bundles
    assert_eq!(g.bundles.len(), 3, "{}", format_bundle_assets(&g));

    // bundle edges should include parallel + conditional
    assert!(g.bundle_edges.iter().any(|(a, b, t)| {
      a.0 == "entry.js" && b.0 == "a.js" && matches!(t, types::IdealEdgeType::Parallel)
    }));
    assert!(g.bundle_edges.iter().any(|(a, b, t)| {
      a.0 == "entry.js" && b.0 == "b.js" && matches!(t, types::IdealEdgeType::Conditional)
    }));
  }

  #[test]
  fn boundary_created_for_type_change() {
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[EdgeSpec::new("entry.js", "styles.css", Priority::Sync)],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);
    assert_eq!(g.bundles.len(), 2, "{}", format_bundle_assets(&g));
    assert!(g.bundle_edges.iter().any(|(a, b, t)| {
      a.0 == "entry.js" && b.0 == "styles.css" && matches!(t, types::IdealEdgeType::Sync)
    }));
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    // At least the two entry bundles exist.
    assert!(
      g.bundles
        .contains_key(&types::IdealBundleId("entry-a.js".into()))
    );
    assert!(
      g.bundles
        .contains_key(&types::IdealBundleId("entry-b.js".into()))
    );
  }

  #[test]
  fn shared_bundle_groups_multiple_assets_for_same_root_set() {
    // Both a and b import react + react-dom
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    // Both shared assets should be in the same (single) shared bundle.
    let shared_bundles: Vec<_> = g
      .bundles
      .iter()
      .filter(|(id, _)| id.0.starts_with("@@shared:"))
      .collect();

    assert_eq!(shared_bundles.len(), 1, "{}", format_bundle_assets(&g));
    let shared_assets = &shared_bundles[0].1.assets;
    assert!(shared_assets.contains("react.js"));
    assert!(shared_assets.contains("react-dom.js"));
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);
    assert_eq!(g.bundles.len(), 1, "{}", format_bundle_assets(&g));
    assert_bundles(&g, &[&["entry.js", "a.js", "b.js"]]);
  }

  #[test]
  fn dominator_diamond_has_single_owner() {
    // entry -> a -> (b,c) -> d
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);
    assert_eq!(g.bundles.len(), 1, "{}", format_bundle_assets(&g));
  }

  #[test]
  fn two_different_root_sets_create_two_shared_bundles() {
    // entry -> lazy a,b,c
    // a,b share x ; b,c share y
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    let shared_bundles: Vec<_> = g
      .bundles
      .iter()
      .filter(|(id, _)| id.0.starts_with("@@shared:"))
      .collect();

    assert_eq!(shared_bundles.len(), 2, "{}", format_bundle_assets(&g));
  }

  #[test]
  fn shared_bundle_edges_are_deduped() {
    // Causes shared bundle and ensures we don't create duplicate parent->shared edges.
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    let shared_id = g
      .bundles
      .keys()
      .find(|id| id.0.starts_with("@@shared:"))
      .expect("shared bundle exists")
      .0
      .clone();

    let count = g
      .bundle_edges
      .iter()
      .filter(|(from, to, _)| from.0 == "entry.js" && to.0 == shared_id)
      .count();

    assert_eq!(count, 1, "{}", format_bundle_assets(&g));
  }

  #[test]
  fn availability_is_intersection_of_parents() {
    // entry -> lazy a, entry -> lazy b
    // a -> sync x, b -> sync y
    // a -> lazy c, b -> lazy c (c has two parents)
    //
    // Availability for c should be intersection of (assets available at a) and (assets available at b).
    // With disjoint sync trees, the intersection should be empty (or only common ancestor assets).
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    let c_bundle = g
      .bundles
      .get(&types::IdealBundleId("c.js".into()))
      .expect("c bundle exists");

    // Both x.js and y.js should not be in the intersection.
    assert!(
      !c_bundle.ancestor_assets.contains("x.js"),
      "{}",
      format_bundle_assets(&g)
    );
    assert!(
      !c_bundle.ancestor_assets.contains("y.js"),
      "{}",
      format_bundle_assets(&g)
    );
  }

  #[test]
  fn shared_extraction_is_suppressed_when_asset_already_available() {
    // entry -> sync vendor
    // entry -> lazy a, entry -> lazy b
    // a -> sync vendor, b -> sync vendor
    //
    // Because vendor is already in the entry bundle, it should be available to both async bundles,
    // so it should NOT be extracted into a shared bundle.
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    let shared_count = g
      .bundles
      .keys()
      .filter(|id| id.0.starts_with("@@shared:"))
      .count();
    assert_eq!(shared_count, 0, "{}", format_bundle_assets(&g));

    // vendor should still be in the entry bundle.
    let entry_bundle = g
      .bundles
      .get(&types::IdealBundleId("entry.js".into()))
      .expect("entry bundle exists");
    assert!(
      entry_bundle.assets.contains("vendor.js"),
      "{}",
      format_bundle_assets(&g)
    );
  }

  #[test]
  fn root_assets_are_never_moved_into_shared_bundles() {
    // a and b are lazy roots and also referenced by a third root c.
    // This should never result in a shared bundle containing a.js or b.js.
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    // Root bundles must still contain themselves.
    for root in ["a.js", "b.js", "c.js"] {
      let b = g
        .bundles
        .get(&types::IdealBundleId(root.into()))
        .expect("root bundle exists");
      assert!(b.assets.contains(root), "{}", format_bundle_assets(&g));
    }

    for b in g.bundles.values() {
      if b.id.0.starts_with("@@shared:") {
        assert!(!b.assets.contains("a.js"), "{}", format_bundle_assets(&g));
        assert!(!b.assets.contains("b.js"), "{}", format_bundle_assets(&g));
      }
    }
  }

  #[test]
  fn isolated_dependency_prevents_sync_traversal_and_shared_extraction() {
    // entry -> lazy a, entry -> lazy b
    // a -> (sync isolated) react, b -> (sync) react
    // The isolated edge should prevent reachability from a to react, so react won't be extracted.
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_invariants(&g);

    // No shared bundles should be created since only one root can reach react.
    let shared_count = g
      .bundles
      .keys()
      .filter(|id| id.0.starts_with("@@shared:"))
      .count();
    assert_eq!(shared_count, 0, "{}", format_bundle_assets(&g));

    // react stays in exactly one of the roots.
    let bundles_containing_react = g
      .bundles
      .values()
      .filter(|b| b.assets.contains("react.js"))
      .count();
    assert_eq!(bundles_containing_react, 1, "{}", format_bundle_assets(&g));
  }
}
