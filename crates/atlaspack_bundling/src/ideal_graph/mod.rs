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

      // Create a bundle group for async bundles (not shared bundles).
      // Shared bundles (root_asset_id == None) are added to existing bundle groups later.
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
            if let Some(target_asset) = asset_graph.get_asset(&neighbor)
              && target_asset.id == *root_asset_id
              && let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id)
            {
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
                if let Some(target_asset) = asset_graph.get_asset(&neighbor)
                  && target_asset.id == *root_asset_id
                  && let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id)
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

    // Wire shared bundles into the bundle groups of the async bundles that depend on them.
    // Shared bundles don't have their own bundle groups - they're siblings in existing groups.
    for (ideal_bundle_id, ideal_bundle) in &ideal_graph.bundles {
      if ideal_bundle.root_asset_id.is_some() {
        continue; // Not a shared bundle.
      }

      let Some(&shared_bundle_node_id) = materialized_bundle_nodes.get(&ideal_bundle_id.0) else {
        continue;
      };

      // Find which bundles depend on this shared bundle via bundle_edges.
      for (from_id, to_id, _edge_type) in &ideal_graph.bundle_edges {
        if to_id != ideal_bundle_id {
          continue;
        }

        // `from_id` is a bundle that depends on this shared bundle.
        // Find that bundle's root asset and its bundle group.
        if let Some(from_bundle) = ideal_graph.bundles.get(from_id)
          && let Some(from_root_asset_id) = &from_bundle.root_asset_id
        {
          let bg_key = format!("bundle_group:{}{}", default_target.name, from_root_asset_id);
          if let Some(&bg_node_id) = bundle_graph.get_node_id_by_content_key(&bg_key) {
            // Add the shared bundle as a sibling in this bundle group.
            bundle_graph.add_edge(
              &bg_node_id,
              &shared_bundle_node_id,
              NativeBundleGraphEdgeType::Null,
            );
            bundle_graph.add_edge(
              &bg_node_id,
              &shared_bundle_node_id,
              NativeBundleGraphEdgeType::Bundle,
            );
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
                if let Some(target_asset) = asset_graph.get_asset(&target_node_id)
                  && let Some(target_bundle_id) = ideal_graph.asset_to_bundle.get(&target_asset.id)
                  && target_bundle_id != ideal_bundle_id
                  && let Some(target_bundle) = ideal_graph.bundles.get(target_bundle_id)
                  && let Some(root_asset_id) = &target_bundle.root_asset_id
                {
                  let bg_key = format!("bundle_group:{}{}", default_target.name, root_asset_id);
                  if let Some(&bg_node_id) = bundle_graph.get_node_id_by_content_key(&bg_key) {
                    bundle_graph.add_edge(
                      &bundle_node_id,
                      &bg_node_id,
                      NativeBundleGraphEdgeType::Bundle,
                    );
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
              if let Some(dep) = asset_graph.get_dependency(&dep_node_id)
                && let Some(&dep_bg_node_id) = bundle_graph.get_node_id_by_content_key(&dep.id)
              {
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

  let hash_reference = format!("HASH_REF_{}", &bundle_id[..16]);

  let entry_asset_id = ideal_bundle.root_asset_id.clone();
  let is_shared_bundle = entry_asset_id.is_none();

  let bundle = Bundle {
    id: bundle_id.clone(),
    public_id: None,
    hash_reference,
    bundle_type: ideal_bundle.bundle_type.clone(),
    env: (*target.env).clone(),
    entry_asset_ids: entry_asset_id.clone().into_iter().collect(),
    main_entry_id: entry_asset_id,
    // Shared bundles don't need stable names - they're loaded as siblings.
    needs_stable_name: Some(if is_shared_bundle {
      false
    } else {
      entry_dep.needs_stable_name
    }),
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
    let mut bundle_ids: Vec<&str> = g.bundles.keys().map(|b| b.0.as_str()).collect();
    bundle_ids.sort();
    let mut out = String::new();
    for bundle_id in bundle_ids {
      let bundle = &g.bundles[&types::IdealBundleId(bundle_id.to_string())];
      let mut assets: Vec<&str> = bundle.assets.iter().map(|s| s.as_str()).collect();
      assets.sort();
      out.push_str(&format!("  {} => [{}]\n", bundle_id, assets.join(", ")));
    }
    if !g.bundle_edges.is_empty() {
      out.push_str("  edges:\n");
      for (from, to, ty) in &g.bundle_edges {
        out.push_str(&format!("    {} {:?} {}\n", from.0, ty, to.0));
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
      .bundles
      .values()
      .map(|b| {
        let mut assets: Vec<String> = b.assets.iter().cloned().collect();
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
          let id = types::IdealBundleId(name.clone());
          if g.bundles.contains_key(&id) {
            vec![&g.bundles.get_key_value(&id).unwrap().0]
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
            .bundles
            .iter()
            .filter(|(id, _)| id.0.starts_with("@@shared:"))
            .filter(|(_, bundle)| {
              let mut actual_assets: Vec<String> = bundle.assets.iter().cloned().collect();
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

  #[test]
  fn entry_with_async_boundary() {
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[EdgeSpec::new("entry.js", "async.js", Priority::Lazy)],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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

  #[derive(Clone, Debug)]
  struct AssetOptions {
    is_bundle_splittable: bool,
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
        }
        let node = asset_graph.add_asset(Arc::new(asset), false);
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
        format_bundle_snapshot(g)
      );
    }

    // No asset appears in more than one splittable bundle.
    // Duplication across entry-like bundles (entries, non-splittable, isolated,
    // needs-stable-name) is expected -- they must be self-contained.
    // For simplicity, we allow duplication across any non-shared rooted bundles.
    let entry_like_bundles: std::collections::HashSet<&str> = g
      .bundles
      .values()
      .filter(|b| b.root_asset_id.is_some())
      .map(|b| b.id.0.as_str())
      .collect();

    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (bundle_id, bundle) in &g.bundles {
      for asset in &bundle.assets {
        if let Some(prev) = seen.insert(asset, &bundle_id.0) {
          // Allow duplication if both bundles are entry-like (non-shared).
          let both_entry_like =
            entry_like_bundles.contains(prev) && entry_like_bundles.contains(bundle_id.0.as_str());
          if !both_entry_like {
            panic!(
              "asset appears in multiple bundles unexpectedly: {asset} in {prev} and {}\n{}",
              bundle_id.0,
              format_bundle_snapshot(g)
            );
          }
        }
      }
    }
  }

  #[test]
  fn creates_shared_bundle_for_asset_needed_by_multiple_async_roots() {
    // entry -> lazy a, lazy b; both a and b sync-import react
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
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[EdgeSpec::new("entry.js", "styles.css", Priority::Sync)],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js"   => ["entry.js"],
        "styles.css" => ["styles.css"],
      },
      edges: {
        "entry.js" sync "styles.css",
      },
    });
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

    assert_graph!(g, {
      bundles: {
        "entry-a.js" => ["entry-a.js", "a.js"],
        "entry-b.js" => ["entry-b.js", "b.js"],
      },
    });
  }

  #[test]
  fn shared_bundle_groups_multiple_assets_for_same_root_set() {
    // Both a and b import react + react-dom -> single shared bundle
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

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        shared(vendor) => ["react.js", "react-dom.js"],
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js", "c.js", "d.js"],
      },
    });
  }

  #[test]
  fn two_different_root_sets_create_two_shared_bundles() {
    // entry -> lazy a,b,c
    // a,b share x ; b,c share y — two separate shared bundles
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

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
        "c.js"     => ["c.js"],
        shared(x) => ["x.js"],
        shared(y) => ["y.js"],
      },
    });
  }

  #[test]
  fn shared_bundle_edges_are_deduped() {
    // Same as shared bundle test — verify only one edge from entry to shared
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
        },
      )],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
    // Since b.js is already in the entry bundle (sync reachable), the async
    // bundle for b.js is redundant and should be internalized (deleted).
    let asset_graph = fixture_graph(
      &["entry.js"],
      &[
        EdgeSpec::new("entry.js", "a.js", Priority::Sync),
        EdgeSpec::new("a.js", "b.js", Priority::Lazy),
        EdgeSpec::new("entry.js", "b.js", Priority::Sync),
      ],
    );

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // b.js should NOT exist as a separate bundle.
    // It should be in the entry bundle since it's sync-reachable.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "a.js", "b.js"],
      },
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    // c.js should be in a shared bundle, not internalized.
    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js" => ["a.js"],
        "b.js" => ["b.js"],
        shared(c) => ["c.js"],
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js"],
        "a.js"     => ["a.js", "x.js"],
        "b.js"     => ["b.js"],
      },
      edges: {
        "entry.js" lazy "a.js",
        "entry.js" lazy "b.js",
        "b.js" lazy "a.js",
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_graph!(g, {
      bundles: {
        "entry.js" => ["entry.js", "vendor.js"],
        "a.js"     => ["a.js"],
        "b.js"     => ["b.js"],
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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
    let (g, _stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

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
}
