use std::collections::HashMap;
use std::sync::Arc;

use atlaspack_core::asset_graph::propagate_requested_symbols_vec;
use atlaspack_core::asset_graph::DependencyNode;
use atlaspack_core::asset_graph::DependencyState;
use atlaspack_core::types::Asset;
use petgraph::graph::NodeIndex;

use super::super::Compilation;
use super::add_asset_dependencies::add_asset_dependencies;
use super::get_direct_discovered_assets::get_direct_discovered_assets;
use super::hashes::AssetHash;
use super::resolve::resolve;
use super::resolve::ResolutionResult;
use super::transform;
use super::TransformationResult;

pub async fn resolve_and_transform(
  Compilation {
    entry_dependencies,
    plugins,
    asset_graph,
    project_root,
    fs,
    ..
  }: &mut Compilation,
) -> anyhow::Result<()> {
  let mut queue = Vec::from_iter(entry_dependencies.drain(0..));
  let mut asset_hash_to_idx = HashMap::<u64, NodeIndex>::new();

  while let Some((dependency_idx, dependency)) = queue.pop() {
    // Resolution (Thread)
    let (path, code, pipeline, query, side_effects) = {
      let resolution = resolve(dependency.clone(), plugins.clone()).await?;

      let DependencyNode {
        dependency,
        requested_symbols,
        state,
      } = asset_graph
        .get_dependency_node_mut(&dependency_idx)
        .unwrap();

      let ResolutionResult::Resolved {
        can_defer,
        path,
        code,
        pipeline,
        query,
        side_effects,
      } = resolution
      else {
        *state = DependencyState::Excluded;
        continue;
      };

      if !side_effects && can_defer && requested_symbols.is_empty() && dependency.symbols.is_some()
      {
        *state = DependencyState::Deferred;
        continue;
      }

      *state = DependencyState::Resolved;
      (path, code, pipeline, query, side_effects)
    };

    // Calculate incoming Asset value based on its starting hash.
    // If it exists then link it to the dependency otherwise
    // create a new asset and link it
    let asset_hash = AssetHash::calculate(
      project_root.clone(),
      code.clone(),
      dependency.env.clone(),
      path.clone(),
      pipeline.clone(),
      query.clone(),
      side_effects.clone(),
    );

    if let Some(asset_idx) = asset_hash_to_idx.get(&asset_hash) {
      if asset_graph.has_edge(&dependency_idx, asset_idx) {
        continue;
      };
      asset_graph.add_edge(&dependency_idx, &asset_idx);
      for next in propagate_requested_symbols_vec(asset_graph, *asset_idx, dependency_idx) {
        queue.push(next);
      }
      continue;
    }

    let asset_idx = asset_graph.add_asset(Asset::default());
    asset_hash_to_idx.insert(asset_hash, asset_idx);
    asset_graph.add_edge(&dependency_idx, &asset_idx);

    // Transformation (Thread)
    let TransformationResult {
      asset,
      discovered_assets,
      dependencies,
    } = {
      transform(
        fs.clone(),
        &project_root,
        plugins.clone(),
        code,
        dependency.env.clone(),
        path.clone(),
        pipeline.clone(),
        query.clone(),
        side_effects,
      )
      .await?
    };

    {
      let asset_node = asset_graph.get_asset_node_mut(&asset_idx).unwrap();
      asset_node.asset = asset.clone();
    }

    let mut added_discovered_assets: HashMap<String, NodeIndex> = HashMap::new();
    let direct_discovered_assets = get_direct_discovered_assets(&discovered_assets, &dependencies);

    for discovered_asset in direct_discovered_assets {
      let asset_idx = asset_graph.add_asset(discovered_asset.asset.clone());

      asset_graph.add_edge(&dependency_idx, &asset_idx);

      for next in add_asset_dependencies(
        asset_graph,
        &discovered_asset.dependencies,
        &discovered_assets,
        asset_idx,
        &mut added_discovered_assets,
        (&asset, asset_idx),
      ) {
        queue.push(next);
      }

      for next in propagate_requested_symbols_vec(asset_graph, asset_idx, dependency_idx) {
        queue.push(next);
      }
    }

    for next in add_asset_dependencies(
      asset_graph,
      &dependencies,
      &discovered_assets,
      asset_idx,
      &mut added_discovered_assets,
      (&asset, asset_idx),
    ) {
      queue.push(next);
    }

    for next in propagate_requested_symbols_vec(asset_graph, asset_idx, dependency_idx) {
      queue.push(next);
    }
  }

  Ok(())
}
