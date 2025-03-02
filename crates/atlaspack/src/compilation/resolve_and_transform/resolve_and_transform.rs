use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::asset_graph::propagate_requested_symbols_vec;
use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::asset_graph::DependencyNode;
use atlaspack_core::asset_graph::DependencyState;
use atlaspack_core::profiler;
use atlaspack_core::types::Asset;
use atlaspack_core::types::Dependency;
use atlaspack_filesystem::FileSystemRef;
use petgraph::graph::NodeIndex;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use super::super::Compilation;
use super::add_asset_dependencies::add_asset_dependencies;
use super::get_direct_discovered_assets::get_direct_discovered_assets;
use super::hashes::AssetHash;
use super::resolve::resolve;
use super::resolve::ResolutionResult;
use super::transform;
use super::TransformationResult;
use crate::plugins::PluginsRef;

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
  let mut jobs = JoinSet::<anyhow::Result<Vec<(NodeIndex, Arc<Dependency>)>>>::new();

  jobs.spawn({
    let entry_dependencies = entry_dependencies.clone();
    async move { Ok(entry_dependencies) }
  });

  let asset_hash_to_idx = Arc::new(Mutex::new(HashMap::<u64, NodeIndex>::new()));
  let asset_graph_sync = Arc::new(Mutex::new(std::mem::take(asset_graph)));

  profiler::time("complete");

  while let Some(Ok(Ok(mut dependencies))) = jobs.join_next().await {
    while let Some((dependency_idx, dependency)) = dependencies.pop() {
      let plugins = plugins.clone();
      let asset_graph_sync = asset_graph_sync.clone();
      let asset_hash_to_idx = asset_hash_to_idx.clone();
      let project_root = project_root.clone();
      let fs = fs.clone();

      jobs.spawn(task(
        dependency_idx,
        dependency,
        asset_graph_sync,
        asset_hash_to_idx,
        plugins,
        project_root,
        fs,
      ));
    }
  }
  profiler::lap("complete");

  std::mem::swap(
    asset_graph,
    &mut *Arc::try_unwrap(asset_graph_sync).unwrap().lock().await,
  );

  profiler::report("complete").log_total();
  println!("");
  profiler::report("task").log_total();
  profiler::report("task").log_average();
  println!("");
  profiler::report("task_long").log_total();
  profiler::report("task_long").log_average();
  println!("");
  profiler::report("task_early").log_total();
  profiler::report("task_early").log_average();
  println!("");
  println!("");
  profiler::report("resolution").log_total();
  profiler::report("resolution").log_average();
  println!("");
  profiler::report("asset_graph1").log_total();
  profiler::report("asset_graph1").log_average();
  println!("");
  profiler::report("transform").log_total();
  profiler::report("transform").log_average();
  println!("");
  profiler::report("asset_graph2").log_total();
  profiler::report("asset_graph2").log_average();

  Ok(())
}

async fn task(
  dependency_idx: NodeIndex,
  dependency: Arc<Dependency>,
  asset_graph_sync: Arc<Mutex<AssetGraph>>,
  asset_hash_to_idx: Arc<Mutex<HashMap<u64, NodeIndex>>>,
  plugins: PluginsRef,
  project_root: PathBuf,
  fs: FileSystemRef,
) -> anyhow::Result<Vec<(NodeIndex, Arc<Dependency>)>> {
  profiler::time("task");
  profiler::time("task_long");
  profiler::time("task_early");

  profiler::time("resolution");
  let resolution = resolve(dependency.clone(), plugins.clone()).await?;
  profiler::lap("resolution");

  let ResolutionResult::Resolved {
    can_defer,
    path,
    code,
    pipeline,
    query,
    side_effects,
  } = resolution
  else {
    asset_graph_sync
      .lock()
      .await
      .get_dependency_node_mut(&dependency_idx)
      .unwrap()
      .state = DependencyState::Excluded;
    profiler::lap("task");
    profiler::lap("task_early");
    return Ok(vec![]);
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

  profiler::time("asset_graph1");
  let (path, code, pipeline, query, side_effects, asset_idx) = {
    let mut asset_graph = asset_graph_sync.lock().await;
    let mut asset_hash_to_idx = asset_hash_to_idx.lock().await;

    let DependencyNode {
      dependency,
      requested_symbols,
      state,
    } = asset_graph
      .get_dependency_node_mut(&dependency_idx)
      .unwrap();

    if !side_effects && can_defer && requested_symbols.is_empty() && dependency.symbols.is_some() {
      *state = DependencyState::Deferred;
      profiler::lap("task");
      profiler::lap("task_early");
      return Ok(vec![]);
    }

    *state = DependencyState::Resolved;

    if let Some(asset_idx) = asset_hash_to_idx.get(&asset_hash) {
      if asset_graph.has_edge(&dependency_idx, asset_idx) {
        profiler::lap("task");
        profiler::lap("task_early");
        return Ok(vec![]);
      };
      asset_graph.add_edge(&dependency_idx, &asset_idx);
      let v = propagate_requested_symbols_vec(&mut asset_graph, *asset_idx, dependency_idx);
      profiler::lap("task");
      profiler::lap("task_early");
      return Ok(v);
    }

    let asset_idx = asset_graph.add_asset(Asset::default());
    asset_hash_to_idx.insert(asset_hash, asset_idx);
    asset_graph.add_edge(&dependency_idx, &asset_idx);

    (path, code, pipeline, query, side_effects, asset_idx)
  };
  profiler::lap("asset_graph1");

  profiler::time("transform");
  let TransformationResult {
    asset,
    discovered_assets,
    dependencies,
  } = transform(
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
  .await?;
  profiler::lap("transform");

  profiler::time("asset_graph2");

  {
    let mut asset_graph = asset_graph_sync.lock().await;

    {
      let asset_node = asset_graph.get_asset_node_mut(&asset_idx).unwrap();
      asset_node.asset = asset.clone();
    }

    let mut added_discovered_assets: HashMap<String, NodeIndex> = HashMap::new();
    let direct_discovered_assets = get_direct_discovered_assets(&discovered_assets, &dependencies);

    let mut next = vec![];

    for discovered_asset in direct_discovered_assets {
      let asset_idx = asset_graph.add_asset(discovered_asset.asset.clone());

      asset_graph.add_edge(&dependency_idx, &asset_idx);

      next.extend(add_asset_dependencies(
        &mut asset_graph,
        &discovered_asset.dependencies,
        &discovered_assets,
        asset_idx,
        &mut added_discovered_assets,
        (&asset, asset_idx),
      ));

      next.extend(propagate_requested_symbols_vec(
        &mut asset_graph,
        asset_idx,
        dependency_idx,
      ));
    }

    next.extend(add_asset_dependencies(
      &mut asset_graph,
      &dependencies,
      &discovered_assets,
      asset_idx,
      &mut added_discovered_assets,
      (&asset, asset_idx),
    ));

    next.extend(propagate_requested_symbols_vec(
      &mut asset_graph,
      asset_idx,
      dependency_idx,
    ));

    profiler::lap("asset_graph2");

    profiler::lap("task_long");
    profiler::lap("task");
    return Ok(next);
  };
}
