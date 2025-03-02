use std::collections::HashMap;
use std::sync::Arc;

use atlaspack_core::asset_graph::propagate_requested_symbols_vec;
use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::types::Asset;
use atlaspack_core::types::AssetWithDependencies;
use atlaspack_core::types::Dependency;
use indexmap::IndexMap;
use petgraph::graph::NodeIndex;

pub fn add_asset_dependencies(
  asset_graph: &mut AssetGraph,
  dependencies: &Vec<Dependency>,
  discovered_assets: &Vec<AssetWithDependencies>,
  asset_idx: NodeIndex,
  added_discovered_assets: &mut HashMap<String, NodeIndex>,
  root_asset: (&Asset, NodeIndex),
) -> Vec<(NodeIndex, Arc<Dependency>)> {
  // Connect dependencies of the Asset
  let mut unique_deps: IndexMap<String, Dependency> = IndexMap::new();

  for dependency in dependencies {
    unique_deps
      .entry(dependency.id())
      .and_modify(|d| {
        // This code is an incomplete version of mergeDependencies in packages/core/core/src/Dependency.js
        // Duplicate dependencies can occur when node globals are polyfilled
        // e.g. 'process'. I think ideally we wouldn't end up with two
        // dependencies post-transform but that needs further investigation to
        // resolve and understand...
        d.meta.extend(dependency.meta.clone());
        if let Some(symbols) = d.symbols.as_mut() {
          if let Some(merge_symbols) = dependency.symbols.as_ref() {
            symbols.extend(merge_symbols.clone());
          }
        } else {
          d.symbols = dependency.symbols.clone();
        }
      })
      .or_insert(dependency.clone());
  }

  let mut results = vec![];

  for (_id, dependency) in unique_deps.into_iter() {
    // Check if this dependency points to a discovered_asset
    let discovered_asset = discovered_assets.iter().find(|discovered_asset| {
      discovered_asset
        .asset
        .unique_key
        .as_ref()
        .is_some_and(|key| key == &dependency.specifier)
    });

    // Check if this dependency points to the root asset
    let dep_to_root_asset = root_asset
      .0
      .unique_key
      .as_ref()
      .is_some_and(|key| key == &dependency.specifier);

    let dependency_idx = asset_graph.add_dependency(dependency);
    asset_graph.add_edge(&asset_idx, &dependency_idx);

    if dep_to_root_asset {
      asset_graph.add_edge(&dependency_idx, &root_asset.1);
    }

    // If the dependency points to a dicovered asset then add the asset using the new
    // dep as it's parent
    if let Some(AssetWithDependencies {
      asset,
      dependencies,
    }) = discovered_asset
    {
      let existing_discovered_asset = added_discovered_assets.get(&asset.id);

      if let Some(asset_node_index) = existing_discovered_asset {
        // This discovered_asset has already been added to the graph so we
        // just need to connect the dependency node to the asset node
        asset_graph.add_edge(&dependency_idx, asset_node_index);
      } else {
        // This discovered_asset isn't yet in the graph so we'll need to add
        // it and assign it's dependencies by calling added_discovered_assets
        // recursively.
        let asset_idx = asset_graph.add_asset(asset.clone());
        asset_graph.add_edge(&dependency_idx, &asset_idx);
        added_discovered_assets.insert(asset.id.clone(), asset_idx);

        add_asset_dependencies(
          asset_graph,
          dependencies,
          discovered_assets,
          asset_idx,
          added_discovered_assets,
          root_asset,
        );
        results.extend(propagate_requested_symbols_vec(
          asset_graph,
          asset_idx,
          dependency_idx,
        ));
      }
    }
  }

  results
}
