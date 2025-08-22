use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

use async_trait::async_trait;
use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, DependencyNode},
  bundle_graph::{AssetRef, BundleGraph, BundleGraphBundle, BundleGraphEdge, BundleGraphNode},
  types::{Bundle, BundleBehavior, Environment, Target},
};
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{Dfs, EdgeFiltered, EdgeRef, IntoNodeReferences},
  Direction,
};
use tracing::info;

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::{AssetGraphRequest, RequestResult},
};

use acyclic_asset_graph::*;
use dominator_tree::*;
use simplified_graph::*;

mod acyclic_asset_graph;
mod asset_graph_builder;
mod dominator_tree;
mod simplified_graph;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct BundleGraphRequestOutput {
  pub bundle_graph: BundleGraph,
  pub asset_graph: AssetGraph,
}

#[derive(Debug, Hash, Default)]
pub struct BundleGraphRequest {}

#[async_trait]
impl Request for BundleGraphRequest {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let (RequestResult::AssetGraph(asset_graph), _) =
      request_context.run_request(AssetGraphRequest {}).await?
    else {
      anyhow::bail!("AssetGraphRequest returned a non-AssetGraph result");
    };

    info!(
      num_assets = asset_graph
        .graph
        .nodes()
        .filter(|node| matches!(node, AssetGraphNode::Asset(_)))
        .count(),
      "Bundling graph"
    );

    let bundle_graph = bundle_asset_graph(&asset_graph.graph);

    Ok(ResultAndInvalidations {
      result: RequestResult::BundleGraph(BundleGraphRequestOutput {
        bundle_graph,
        asset_graph: asset_graph.graph,
      }),
      invalidations: vec![],
    })
  }
}

fn bundle_asset_graph(asset_graph: &AssetGraph) -> BundleGraph {
  let simplified_graph = simplify_graph(asset_graph);
  let (root_node, acyclic_graph) = remove_cycles(&simplified_graph);
  let dominator_tree = build_dominator_tree(&acyclic_graph, root_node);

  {
    let mut reachable_assets = HashSet::new();
    let mut dfs = Dfs::new(&dominator_tree, asset_graph.root_node());
    while let Some(node) = dfs.next(&asset_graph.graph) {
      reachable_assets.insert(node);
    }
    for (node_index, asset_node) in
      asset_graph
        .graph
        .node_references()
        .filter_map(|(node_index, node)| match node {
          AssetGraphNode::Asset(asset_node) => Some((node_index, asset_node)),
          _ => None,
        })
    {
      // Validate simplified graph
      assert!(simplified_graph.contains_node(node_index));
      let asset_on_new_graph = simplified_graph.node_weight(node_index);
      assert!(asset_on_new_graph.is_some());
      let SimplifiedAssetGraphNode::Asset(asset_on_new_graph) = asset_on_new_graph.unwrap() else {
        panic!("Asset on new graph is not an asset");
      };
      assert!(asset_on_new_graph.file_path() == asset_node.asset.file_path);

      // Validate dominator graph
      assert!(
        reachable_assets.contains(&node_index),
        "Asset is not reachable from the root: {:#?}",
        asset_node.asset.file_path
      );
    }
  }

  let bundle_graph = make_bundle_graph(MakeBundleGraphParams {
    root_node,
    dominator_tree: &dominator_tree,
  });

  bundle_graph
}

struct MakeBundleGraphParams<'a> {
  root_node: NodeIndex,
  dominator_tree: &'a StableDiGraph<DominatorTreeNode, DominatorTreeEdge>,
}

fn make_bundle_graph(
  MakeBundleGraphParams {
    root_node,
    dominator_tree,
  }: MakeBundleGraphParams,
) -> BundleGraph {
  let mut bundle_graph: StableDiGraph<Option<BundleGraphNode>, BundleGraphEdge> = dominator_tree
    .filter_map(
      |_, node| match node {
        AcyclicAssetGraphNode::Root => Some(Some(BundleGraphNode::Root)),
        _ => Some(None),
      },
      |_, _| None,
    );

  struct AddedBundle {
    bundle_dominator_tree_node_index: NodeIndex,
    bundle_node_index: NodeIndex,
  }

  let mut bundles_by_asset_index = HashMap::new();
  let mut added_bundles = vec![];

  for edge in dominator_tree.edges(root_node) {
    let bundle_edge_type = match edge.weight() {
      DominatorTreeEdge::EntryAssetRoot(_) => Some(BundleGraphEdge::RootEntryOf),
      DominatorTreeEdge::AsyncRoot(_) => Some(BundleGraphEdge::RootAsyncBundleOf),
      DominatorTreeEdge::TypeChangeRoot(_) => Some(BundleGraphEdge::RootTypeChangeBundleOf),
      DominatorTreeEdge::SharedBundleRoot => Some(BundleGraphEdge::RootSharedBundleOf),
      DominatorTreeEdge::ImmediateDominator => None,
      DominatorTreeEdge::AssetDependency(_) => None,
      DominatorTreeEdge::AssetAsyncDependency(_) => None,
    };
    let Some(bundle_edge_type) = bundle_edge_type else {
      continue;
    };

    let root_asset = edge.target();
    let mut assets_in_bundle = vec![];
    let mut bundle_assets: StableDiGraph<AssetRef, ()> = StableDiGraph::new();

    let filtered_dominator_tree =
      EdgeFiltered::from_fn(&dominator_tree, |edge| match edge.weight() {
        DominatorTreeEdge::ImmediateDominator => true,
        _ => false,
      });

    petgraph::visit::depth_first_search(
      &filtered_dominator_tree,
      [root_asset],
      |event| match event {
        petgraph::visit::DfsEvent::Discover(node_index, _) => {
          let node_weight = dominator_tree.node_weight(node_index).unwrap();
          match node_weight {
            AcyclicAssetGraphNode::Asset(asset_node) => {
              assets_in_bundle.push(node_index);

              bundle_assets.add_node(asset_node.clone());
            }
            AcyclicAssetGraphNode::Cycle(assets) => {
              assets_in_bundle.push(node_index);

              for asset in assets {
                assets_in_bundle.push(asset.asset_graph_node_index());
                bundle_assets.add_node(asset.clone());
              }
            }
            _ => {}
          }
        }
        _ => {}
      },
    );

    let AcyclicAssetGraphNode::Asset(entry_asset) = dominator_tree.node_weight(root_asset).unwrap()
    else {
      tracing::debug!(?root_asset, "Bundle entry node is not an asset");
      continue;
    };

    let file_name = entry_asset
      .file_path()
      .file_stem()
      .map(|stem| stem.to_string_lossy().to_string())
      .unwrap_or_else(|| format!("bundle-{}", entry_asset.id()));

    let bundle_id = format!("bundle(entry={})", entry_asset.id());
    let bundle_name = Some(format!(
      "{}.{}",
      file_name,
      entry_asset.file_type().extension()
    ));

    // println!("entry-asset-path: {:?}", entry_asset.file_path());

    let bundle_graph_bundle = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: entry_asset.file_type().clone(),
        entry_asset_ids: vec![entry_asset.id()],
        env: Environment::default(),
        hash_reference: "".to_string(),
        id: bundle_id,
        is_splittable: true,
        main_entry_id: Some(entry_asset.id()),
        manual_shared_bundle: None,
        name: bundle_name,
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: bundle_assets,
    };

    let bundle_node_index =
      bundle_graph.add_node(Some(BundleGraphNode::Bundle(Arc::new(bundle_graph_bundle))));
    bundle_graph.add_edge(root_node, bundle_node_index, bundle_edge_type.clone());

    // println!(
    //   "  >> added bundle {} (bundle {:?})",
    //   bundle_node_index.index(),
    //   bundle_edge_type
    // );
    added_bundles.push(AddedBundle {
      bundle_dominator_tree_node_index: root_asset,
      bundle_node_index,
    });

    for index in assets_in_bundle {
      bundles_by_asset_index.insert(index, bundle_node_index);
    }
  }

  for (node_index, node_weight) in dominator_tree.node_references() {
    match node_weight {
      AcyclicAssetGraphNode::Asset(asset_node) => {
        if !bundles_by_asset_index.contains_key(&node_index) {
          tracing::error!(
            "Asset is not in any bundle:\n --> path={:#?}",
            dominator_tree.node_weight(node_index).unwrap().file_path()
          );
        }
      }
      AcyclicAssetGraphNode::Cycle(assets) => {
        for asset in assets {
          if !bundles_by_asset_index.contains_key(&asset.asset_graph_node_index()) {
            tracing::error!(
              "Asset is not in any bundle:\n --> path={:#?}",
              asset.file_path()
            );
          }
        }
      }
      _ => {}
    }
  }

  // Link the bundles together
  for AddedBundle {
    bundle_dominator_tree_node_index,
    bundle_node_index,
  } in added_bundles.iter()
  {
    let filtered_dominator_tree =
      EdgeFiltered::from_fn(&dominator_tree, |edge| match edge.weight() {
        DominatorTreeEdge::ImmediateDominator => true,
        _ => false,
      });

    petgraph::visit::depth_first_search(
      &filtered_dominator_tree,
      [*bundle_dominator_tree_node_index],
      |event| match event {
        petgraph::visit::DfsEvent::Discover(node_index, _) => {
          for edge in dominator_tree.edges_directed(node_index, Direction::Outgoing) {
            match edge.weight() {
              DominatorTreeEdge::ImmediateDominator => {}
              DominatorTreeEdge::AssetDependency(dependency_node) => {
                let target = edge.target();
                let target_bundle = bundles_by_asset_index[&target];
                //   tracing::error!(
                //     "Asset is not in any bundle:\n --> path={:#?}",
                //     dominator_tree.node_weight(target).unwrap().file_path()
                //   );
                //   continue;
                // };

                if target_bundle == *bundle_node_index {
                  continue;
                }

                bundle_graph.add_edge(
                  *bundle_dominator_tree_node_index,
                  target_bundle,
                  // TODO: Maintain the reason this edge is here, e.g.: carry over the source, target, dependency etc
                  BundleGraphEdge::BundleSyncLoads,
                );
              }
              DominatorTreeEdge::AssetAsyncDependency(dependency_node) => {
                let target = edge.target();
                let target_bundle = bundles_by_asset_index[&target];

                let source_weight = dominator_tree.node_weight(node_index).unwrap();
                let target_weight = dominator_tree.node_weight(target).unwrap();

                bundle_graph.add_edge(
                  *bundle_node_index,
                  target_bundle,
                  // TODO: Maintain the reason this edge is here, e.g.: carry over the source, target, dependency etc
                  BundleGraphEdge::BundleAsyncLoads,
                );
              }
              DominatorTreeEdge::EntryAssetRoot(dependency_node) => unreachable!(),
              DominatorTreeEdge::AsyncRoot(dependency_node) => unreachable!(),
              DominatorTreeEdge::SharedBundleRoot => unreachable!(),
              DominatorTreeEdge::TypeChangeRoot(dependency_node) => unreachable!(),
            }
          }
        }
        _ => {}
      },
    );
  }

  let bundle_graph = bundle_graph.filter_map(|_, node| node.clone(), |_, edge| Some(edge.clone()));

  BundleGraph::build_from(root_node, bundle_graph)
}

#[cfg(test)]
mod tests {
  use std::{
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
  };

  use atlaspack_core::{
    asset_graph::AssetNode,
    bundle_graph::{BundleGraphEdge, BundleRef},
    types::{Asset, FileType},
  };
  use petgraph::{
    algo::toposort,
    dot::dot_parser::{DotAttrList, DotNodeWeight, ParseFromDot},
    graph_from_str,
    visit::{IntoEdgeReferences, IntoNodeReferences},
  };

  use crate::{
    requests::bundle_graph_request::asset_graph_builder::simplified_asset_graph_builder,
    test_utils::{
      get_repo_path,
      graph::{expect_edge, expect_node},
      make_test_atlaspack, setup_test_directory,
    },
  };

  use super::*;

  #[test]
  fn test_make_bundle_graph_over_empty_graph() {
    let dominator_tree = StableDiGraph::new();
    let bundle_graph = make_bundle_graph(MakeBundleGraphParams {
      root_node: NodeIndex::new(0),
      dominator_tree: &dominator_tree,
    });

    assert_eq!(bundle_graph.num_bundles(), 0);
  }

  #[test]
  fn test_make_bundle_graph_over_single_asset_tree() {
    let mut graph = simplified_asset_graph_builder();
    let asset = graph.entry_asset("src/index.ts");
    let graph = graph.build();

    let (root, acyclic_graph) = remove_cycles(&graph);
    let dominator_tree = build_dominator_tree(&acyclic_graph, root);

    let bundle_graph = make_bundle_graph(MakeBundleGraphParams {
      root_node: root,
      dominator_tree: &dominator_tree,
    });

    assert_eq!(
      bundle_graph.num_bundles(),
      1,
      "Single asset bundle graph has one bundle"
    );
    let bundle = bundle_graph.bundles().next().unwrap();
    assert_eq!(bundle.num_assets(), 1);
    let asset = bundle.assets().next().unwrap();
    assert_eq!(asset.file_path(), PathBuf::from("src/index.ts"));

    let root = bundle_graph.root();
    let graph = bundle_graph.graph();
    let edges = graph
      .edges_directed(root, Direction::Outgoing)
      .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1, "Single asset bundle graph has one edge");
    let edge = edges[0];
    assert_eq!(edge.weight(), &BundleGraphEdge::RootEntryOf);
    assert_eq!(edge.source(), root);
    assert_eq!(edge.target(), bundle.node_index());
  }

  #[test]
  fn test_make_bundle_graph_over_single_asset_tree_with_async_dependency() {
    let mut graph = simplified_asset_graph_builder();
    let asset = graph.entry_asset("src/index.ts");
    let dependency = graph.asset("src/dependency.ts");
    graph.async_dependency(asset, dependency);
    let graph = graph.build();

    let (root, acyclic_graph) = remove_cycles(&graph);
    let dominator_tree = build_dominator_tree(&acyclic_graph, root);

    let bundle_graph = make_bundle_graph(MakeBundleGraphParams {
      root_node: root,
      dominator_tree: &dominator_tree,
    });

    assert_eq!(
      bundle_graph.num_bundles(),
      2,
      "Bundle graph should have two bundles"
    );

    let graph = bundle_graph.graph();
    let index_bundle = expect_bundle(&bundle_graph, "src/index.ts");
    let async_bundle = expect_bundle(&bundle_graph, "src/dependency.ts");

    assert_eq!(index_bundle.num_assets(), 1);
    assert_eq!(async_bundle.num_assets(), 1);

    let edges = graph
      .edges_connecting(bundle_graph.root(), index_bundle.node_index())
      .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].weight(), &BundleGraphEdge::RootEntryOf);

    let edges = graph
      .edges_connecting(bundle_graph.root(), async_bundle.node_index())
      .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].weight(), &BundleGraphEdge::RootAsyncBundleOf);

    let edges = graph
      .edges_connecting(index_bundle.node_index(), async_bundle.node_index())
      .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].weight(), &BundleGraphEdge::BundleAsyncLoads);
  }

  #[test]
  fn test_nested_synchronous_asset_chain_is_bundled_together() {
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let b = graph.asset("src/b.ts");
    let c = graph.asset("src/c.ts");
    graph.sync_dependency(a, b);
    graph.sync_dependency(b, c);

    let graph = graph.build();

    let (root, acyclic_graph) = remove_cycles(&graph);
    let dominator_tree = build_dominator_tree(&acyclic_graph, root);

    let bundle_graph = make_bundle_graph(MakeBundleGraphParams {
      root_node: root,
      dominator_tree: &dominator_tree,
    });

    assert_eq!(
      bundle_graph.num_bundles(),
      1,
      "Bundle graph should have two bundles"
    );

    let graph = bundle_graph.graph();
    let bundle = expect_bundle(&bundle_graph, "src/a.ts");
    assert_eq!(bundle.num_assets(), 3);

    let edges = graph
      .edges_connecting(bundle_graph.root(), bundle.node_index())
      .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].weight(), &BundleGraphEdge::RootEntryOf);

    let mut assets = bundle.assets().collect::<Vec<_>>();
    assets.sort_by_key(|asset| asset.file_path());
    assert_eq!(assets.len(), 3);
    assert_eq!(assets[0].file_path(), PathBuf::from("src/a.ts"));
    assert_eq!(assets[1].file_path(), PathBuf::from("src/b.ts"));
    assert_eq!(assets[2].file_path(), PathBuf::from("src/c.ts"));
  }

  mod unit_tests {
    use atlaspack_core::types::{Asset, Dependency};

    use super::*;

    #[test]
    fn test_acyclic_graph_maintains_the_same_structure_as_the_original_graph() {
      //    root -> entry<dep> -> a -> dep(a:b) -> b
      // becomes
      //    root ---- (dep) ----> a --- (dep) ---> b
      //       \----------------------------------/
      let mut simplified_graph = SimplifiedAssetGraph::new();
      let root = simplified_graph.add_node(SimplifiedAssetGraphNode::Root);
      let a = simplified_graph.add_node(SimplifiedAssetGraphNode::Asset(AssetRef::new(
        AssetNode::default(),
        NodeIndex::new(0),
      )));
      let b = simplified_graph.add_node(SimplifiedAssetGraphNode::Asset(AssetRef::new(
        AssetNode::default(),
        NodeIndex::new(1),
      )));

      simplified_graph.add_edge(
        root,
        a,
        SimplifiedAssetGraphEdge::EntryAssetRoot(DependencyNode::default()),
      );
      simplified_graph.add_edge(
        root,
        b,
        SimplifiedAssetGraphEdge::AsyncRoot(DependencyNode::default()),
      );
      simplified_graph.add_edge(
        a,
        b,
        SimplifiedAssetGraphEdge::AssetAsyncDependency(DependencyNode::default()),
      );

      let (root, acyclic_graph) = remove_cycles(&simplified_graph);
      let find = |graph: &AcyclicAssetGraph, index: NodeIndex| {
        graph
          .node_references()
          .find_map(|(new_index, node)| {
            if let AcyclicAssetGraphNode::Asset(asset_ref) = node {
              if asset_ref.asset_graph_node_index() == index {
                Some((new_index, asset_ref.clone()))
              } else {
                None
              }
            } else {
              None
            }
          })
          .unwrap()
      };
      let (a, _) = find(&acyclic_graph, NodeIndex::new(0));
      let (b, _) = find(&acyclic_graph, NodeIndex::new(1));

      assert_eq!(acyclic_graph.node_count(), 3);
      assert_eq!(acyclic_graph.edge_count(), 3);

      assert_eq!(
        acyclic_graph.node_weight(root).unwrap(),
        &AcyclicAssetGraphNode::Root
      );

      assert!(matches!(
        acyclic_graph
          .edges_connecting(root, a)
          .next()
          .unwrap()
          .weight(),
        SimplifiedAssetGraphEdge::EntryAssetRoot(_)
      ));

      assert!(matches!(
        acyclic_graph
          .edges_connecting(a, b)
          .next()
          .unwrap()
          .weight(),
        SimplifiedAssetGraphEdge::AssetAsyncDependency(_)
      ));

      assert!(matches!(
        acyclic_graph
          .edges_connecting(root, b)
          .next()
          .unwrap()
          .weight(),
        SimplifiedAssetGraphEdge::AsyncRoot(_)
      ));
    }
  }

  fn expect_asset<'a, E>(
    dominator_tree: &'a StableDiGraph<DominatorTreeNode, E>,
    project_dir: &Path,
    path: &str,
  ) -> (NodeIndex, &'a DominatorTreeNode) {
    let predicate = move |node: &DominatorTreeNode| -> bool {
      let DominatorTreeNode::Asset(asset_ref) = node else {
        return false;
      };

      asset_ref.file_path() == &project_dir.join(path)
    };

    expect_node(&dominator_tree, predicate)
  }

  fn expect_bundle(bundle_graph: &BundleGraph, path: &str) -> BundleRef {
    bundle_graph
      .bundles()
      .find(|bundle| {
        bundle
          .assets()
          .find(|asset| asset.file_path() == PathBuf::from(path))
          .is_some()
      })
      .unwrap()
  }

  fn expect_cycle<'a, E>(
    dominator_tree: &'a StableDiGraph<DominatorTreeNode, E>,
    project_dir: &Path,
    path: &str,
  ) -> (NodeIndex, &'a DominatorTreeNode) {
    let predicate = move |node: &DominatorTreeNode| -> bool {
      let DominatorTreeNode::Cycle(asset_refs) = node else {
        return false;
      };

      asset_refs
        .iter()
        .any(|asset_ref| asset_ref.file_path() == &project_dir.join(path))
    };

    expect_node(&dominator_tree, predicate)
  }

  fn asset_path_label(asset_node: &AssetNode, repo_root: &Path, project_dir: &Path) -> String {
    let file_path = asset_node.asset.file_path.clone();
    let path = if file_path.starts_with(repo_root) {
      PathBuf::from("<atlaspack_repo_root>").join(file_path.strip_prefix(repo_root).unwrap())
    } else if file_path.starts_with(project_dir) {
      file_path.strip_prefix(project_dir).unwrap().to_path_buf()
    } else {
      file_path.clone()
    };

    format!("{}", path.display())
  }

  fn asset_node_label(asset_node: &AssetNode, repo_root: &Path, project_dir: &Path) -> String {
    let path = asset_path_label(asset_node, repo_root, project_dir);
    format!("Asset({})", path)
  }

  fn simplified_asset_graph_node_label(
    node: &SimplifiedAssetGraphNode,
    repo_root: &Path,
    project_dir: &Path,
  ) -> String {
    match node {
      SimplifiedAssetGraphNode::Asset(asset_node) => {
        asset_node_label(asset_node, repo_root, project_dir)
      }
      SimplifiedAssetGraphNode::Root => "Root".to_string(),
    }
  }

  fn make_dot_graph<N, E>(
    graph: &StableDiGraph<N, E>,
    node_label_fn: impl Fn(&N) -> String,
    edge_label_fn: impl Fn(&E) -> String,
    is_node_visible: impl Fn(&N) -> bool,
  ) -> StableDiGraph<String, String> {
    let mut result = StableDiGraph::new();

    let mut indexes = HashMap::new();

    if let Ok(sorted_nodes) = toposort(&graph, None) {
      for node_index in &sorted_nodes {
        let node = graph.node_weight(*node_index).unwrap();
        let label = node_label_fn(node);
        let new_index = result.add_node(label);
        indexes.insert(*node_index, new_index);
      }

      for node_index in sorted_nodes {
        for edge in graph.edges_directed(node_index, Direction::Outgoing) {
          let source_node_index = edge.source();
          let target_node_index = edge.target();
          let source_node_index = *indexes.get(&source_node_index).unwrap();
          let target_node_index = *indexes.get(&target_node_index).unwrap();

          let edge_label = edge_label_fn(edge.weight());
          result.add_edge(source_node_index, target_node_index, edge_label);
        }
      }
    } else {
      for node_index in graph.node_indices() {
        let node = graph.node_weight(node_index).unwrap();
        let label = node_label_fn(node);
        let new_index = result.add_node(label);
        indexes.insert(node_index, new_index);
      }

      for edge in graph.edge_references() {
        let source_node_index = edge.source();
        let target_node_index = edge.target();
        let source_node_index = *indexes.get(&source_node_index).unwrap();
        let target_node_index = *indexes.get(&target_node_index).unwrap();

        let edge_label = edge_label_fn(edge.weight());
        result.add_edge(source_node_index, target_node_index, edge_label);
      }
    }

    for node_index in graph.node_indices() {
      if !is_node_visible(graph.node_weight(node_index).unwrap()) {
        result.remove_node(node_index);
      }
    }

    result
  }

  type DebugGraph = StableDiGraph<String, String>;

  fn simplified_asset_graph_edge_label(edge: &SimplifiedAssetGraphEdge) -> String {
    match edge {
      SimplifiedAssetGraphEdge::EntryAssetRoot(_) => "entry".to_string(),
      SimplifiedAssetGraphEdge::AsyncRoot(_) => "async".to_string(),
      SimplifiedAssetGraphEdge::TypeChangeRoot(_) => "type-change".to_string(),
      SimplifiedAssetGraphEdge::AssetDependency(_) => "sync".to_string(),
      SimplifiedAssetGraphEdge::AssetAsyncDependency(_) => "async".to_string(),
    }
  }

  fn make_simplified_dot_graph(
    project_dir: &Path,
    simplified_graph: &SimplifiedAssetGraph,
  ) -> DebugGraph {
    let repo_root = get_repo_path();
    make_dot_graph(
      simplified_graph,
      |node| simplified_asset_graph_node_label(node, &repo_root, &project_dir),
      |edge| simplified_asset_graph_edge_label(edge),
      |_| true,
    )
  }

  fn acyclic_asset_graph_node_label(
    node: &AcyclicAssetGraphNode,
    repo_root: &Path,
    project_dir: &Path,
  ) -> String {
    match node {
      AcyclicAssetGraphNode::Asset(asset_node) => {
        asset_node_label(asset_node, &repo_root, &project_dir)
      }
      AcyclicAssetGraphNode::Cycle(assets) => {
        let mut cycle_assets = assets
          .iter()
          .map(|asset| asset_path_label(asset, &repo_root, &project_dir))
          .collect::<Vec<_>>();
        cycle_assets.sort();

        format!("Cycle({})", cycle_assets.join(", "))
      }
      AcyclicAssetGraphNode::Root => "Root".to_string(),
    }
  }

  fn make_acyclic_dot_graph(project_dir: PathBuf, acyclic_graph: AcyclicAssetGraph) -> DebugGraph {
    let repo_root = get_repo_path();

    make_dot_graph(
      &acyclic_graph,
      |node| acyclic_asset_graph_node_label(node, &repo_root, &project_dir),
      |edge| simplified_asset_graph_edge_label(edge),
      |_| true,
    )
  }

  fn make_dominator_tree_dot_graph(
    project_dir: PathBuf,
    dominator_tree: StableDiGraph<DominatorTreeNode, DominatorTreeEdge>,
  ) -> DebugGraph {
    let repo_root = get_repo_path();

    make_dot_graph(
      &dominator_tree,
      |node| match node {
        AcyclicAssetGraphNode::Root => "Root".to_string(),
        // AcyclicAssetGraphNode::Entry => "Entry".to_string(),
        AcyclicAssetGraphNode::Asset(asset_node) => {
          asset_node_label(asset_node, &repo_root, &project_dir)
        }
        AcyclicAssetGraphNode::Cycle(assets) => {
          let mut cycle_assets = assets
            .iter()
            .map(|asset| asset_path_label(asset, &repo_root, &project_dir))
            .collect::<Vec<_>>();
          cycle_assets.sort();

          format!("Cycle({})", cycle_assets.join(", "))
        }
      },
      |_| "".to_string(),
      |_| true,
    )
  }

  #[tokio::test(flavor = "multi_thread")]
  #[ignore]
  async fn test_simplify_graph() {
    let project_dir = setup_test_directory("test_simplify_graph").unwrap();
    create_dir_all(project_dir.join("src")).unwrap();
    write(
      project_dir.join("src/foo.ts"),
      r#"export const foo = "foo";"#,
    )
    .unwrap();
    write(
      project_dir.join("src/index.ts"),
      r#"
import { foo } from "./foo";

console.log(foo);
"#,
    )
    .unwrap();
    let atlaspack = make_test_atlaspack(&[project_dir.join("src/index.ts")])
      .await
      .unwrap();
    let asset_graph = atlaspack
      .run_request_async(AssetGraphRequest::default())
      .await
      .unwrap()
      .into_asset_graph()
      .unwrap();

    let simplified_graph = simplify_graph(&asset_graph.graph);

    let dot_graph = make_simplified_dot_graph(&project_dir, &simplified_graph);
    let dot = petgraph::dot::Dot::new(&dot_graph);
    let dot = format!("{}", dot);
    let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    1 [ label = "Asset(src/index.ts)" ]
    2 [ label = "Asset(src/foo.ts)" ]
    0 -> 1 [ label = "entry" ]
    1 -> 2 [ label = "sync" ]
}
    "#;

    println!("dot:\n{}\n\n", dot);
    assert_eq!(dot.trim(), expected_dot.trim());
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_remove_cycles() {
    let project_dir = setup_test_directory("test_remove_cycles").unwrap();
    create_dir_all(project_dir.join("src")).unwrap();
    write(
      project_dir.join("src/foo.ts"),
      r#"
import { bar } from "./bar";

export const foo = "foo" + bar;
"#,
    )
    .unwrap();
    write(
      project_dir.join("src/bar.ts"),
      r#"
import { foo } from "./foo";

export const bar = "bar" + foo;
"#,
    )
    .unwrap();
    write(
      project_dir.join("src/baz.ts"),
      r#"
import { foo } from "./foo";
import { bar } from "./bar";

export const baz = "baz" + foo + bar;
"#,
    )
    .unwrap();

    let atlaspack = make_test_atlaspack(&[project_dir.join("src/baz.ts")])
      .await
      .unwrap();
    let asset_graph = atlaspack
      .run_request_async(AssetGraphRequest::default())
      .await
      .unwrap()
      .into_asset_graph()
      .unwrap();
    let simplified_graph = simplify_graph(&asset_graph.graph);
    let (root, acyclic_graph) = remove_cycles(&simplified_graph);

    let (baz, _) = expect_asset(&acyclic_graph, &project_dir, "src/baz.ts");
    let (cycle, _) = expect_cycle(&acyclic_graph, &project_dir, "src/foo.ts");

    expect_edge(&acyclic_graph, root, baz);
    expect_edge(&acyclic_graph, baz, cycle);
  }

  mod test_dominator_tree {
    use crate::test_utils::graph::{expect_edge, expect_node};

    use super::*;

    async fn build_test_dominator_tree(
      project_dir: &Path,
    ) -> StableDiGraph<DominatorTreeNode, DominatorTreeEdge> {
      let atlaspack = make_test_atlaspack(&[project_dir.join("src/index.ts")])
        .await
        .unwrap();
      let asset_graph = atlaspack
        .run_request_async(AssetGraphRequest::default())
        .await
        .unwrap()
        .into_asset_graph()
        .unwrap();
      let simplified_graph = simplify_graph(&asset_graph.graph);
      let simplified_dot_graph = make_simplified_dot_graph(&project_dir, &simplified_graph);
      let simplified_dot = petgraph::dot::Dot::new(&simplified_dot_graph);
      let simplified_dot = format!("{}", simplified_dot);
      println!("simplified_dot:\n{}\n\n", simplified_dot);

      let (root_node, acyclic_graph) = remove_cycles(&simplified_graph);
      let dominator_tree = build_dominator_tree(&acyclic_graph, root_node);

      dominator_tree
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_build_dominator_tree_with_a_single_file() {
      let project_dir = setup_test_directory("test_build_dominator_tree").unwrap();
      create_dir_all(project_dir.join("src")).unwrap();
      write(
        project_dir.join("src/index.ts"),
        r#"export const foo = "foo";"#,
      )
      .unwrap();

      let dominator_tree = build_test_dominator_tree(&project_dir).await;
      let dot_graph = make_dominator_tree_dot_graph(project_dir, dominator_tree);
      let dot = petgraph::dot::Dot::new(&dot_graph);
      let dot = format!("{}", dot);
      let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    1 [ label = "Asset(src/index.ts)" ]
    0 -> 1 [ label = "" ]
}
    "#;

      println!("dot: \n{}", dot);
      assert_eq!(dot.trim(), expected_dot.trim());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_build_dominator_tree_with_a_single_file_and_a_dependency() {
      let project_dir =
        setup_test_directory("test_build_dominator_tree_with_a_single_file_and_a_dependency")
          .unwrap();
      create_dir_all(project_dir.join("src")).unwrap();
      write(
        project_dir.join("src/index.ts"),
        r#"
import { foo } from "./foo";

export const index = "index" + foo;
"#,
      )
      .unwrap();
      write(
        project_dir.join("src/foo.ts"),
        r#"export const foo = "foo";"#,
      )
      .unwrap();

      let dominator_tree = build_test_dominator_tree(&project_dir).await;

      let dot_graph = make_dominator_tree_dot_graph(project_dir, dominator_tree);
      let dot = petgraph::dot::Dot::new(&dot_graph);
      let dot = format!("{}", dot);
      let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    1 [ label = "Asset(src/index.ts)" ]
    2 [ label = "Asset(src/foo.ts)" ]
    0 -> 1 [ label = "" ]
    1 -> 2 [ label = "" ]
}
    "#;

      println!("dot: \n{}", dot);
      assert_eq!(dot.trim(), expected_dot.trim());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_build_dominator_tree_with_an_async_import() {
      let project_dir =
        setup_test_directory("test_build_dominator_tree_with_an_async_import").unwrap();
      create_dir_all(project_dir.join("src")).unwrap();
      write(
        project_dir.join("src/index.ts"),
        r#"
export const index = async () => {
  const foo = await import("./foo");
  return "index" + foo;
};
"#,
      )
      .unwrap();
      write(
        project_dir.join("src/foo.ts"),
        r#"export const foo = "foo";"#,
      )
      .unwrap();

      let dominator_tree = build_test_dominator_tree(&project_dir).await;

      let dot_graph = make_dominator_tree_dot_graph(project_dir, dominator_tree);
      let dot = petgraph::dot::Dot::new(&dot_graph);
      let dot = format!("{}", dot);
      let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    1 [ label = "Asset(src/index.ts)" ]
    2 [ label = "Asset(src/foo.ts)" ]
    0 -> 1 [ label = "" ]
    0 -> 2 [ label = "" ]
}
    "#;

      println!("dot: \n{}", dot);
      assert_eq!(dot.trim(), expected_dot.trim());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_build_dominator_tree_with_a_shared_bundle() {
      let project_dir =
        setup_test_directory("test_build_dominator_tree_with_a_shared_bundle").unwrap();
      create_dir_all(project_dir.join("src")).unwrap();
      write(
        project_dir.join("src/index.ts"),
        r#"

export const index = async () => {
  const { bar } = await import("./bar");
  const { foo } = await import("./foo");
  return "index" + foo + bar;
};
"#,
      )
      .unwrap();
      write(
        project_dir.join("src/foo.ts"),
        r#"
import { s } from "./s";

export const foo = "foo" + s;
        "#,
      )
      .unwrap();
      write(
        project_dir.join("src/bar.ts"),
        r#"
import { s } from "./s";
import { k } from "./k";

export const bar = "bar" + s + k;
        "#,
      )
      .unwrap();
      write(
        project_dir.join("src/s.ts"),
        r#"
export const s = "s";
        "#,
      )
      .unwrap();
      write(
        project_dir.join("src/k.ts"),
        r#"
export const k = "k";
        "#,
      )
      .unwrap();

      let dominator_tree = build_test_dominator_tree(&project_dir).await;
      let (root, _) = expect_node(&dominator_tree, |node| {
        matches!(node, DominatorTreeNode::Root)
      });
      let (index, _) = expect_asset(&dominator_tree, &project_dir, "src/index.ts");
      let (bar, _) = expect_asset(&dominator_tree, &project_dir, "src/bar.ts");
      let (foo, _) = expect_asset(&dominator_tree, &project_dir, "src/foo.ts");
      let (s, _) = expect_asset(&dominator_tree, &project_dir, "src/s.ts");
      let (k, _) = expect_asset(&dominator_tree, &project_dir, "src/k.ts");

      expect_edge(&dominator_tree, root, index);
      expect_edge(&dominator_tree, root, bar);
      expect_edge(&dominator_tree, root, foo);
      expect_edge(&dominator_tree, root, s);
      expect_edge(&dominator_tree, bar, k);

      assert_eq!(dominator_tree.node_count(), 6);

      let dot_graph = make_dominator_tree_dot_graph(project_dir, dominator_tree);
      let dot = petgraph::dot::Dot::new(&dot_graph);
      let dot = format!("{}", dot);

      //       let expected_dot = r#"
      // digraph {
      //     0 [ label = "Root" ]
      //     1 [ label = "Asset(src/index.ts)" ]
      //     2 [ label = "Asset(src/bar.ts)" ]
      //     3 [ label = "Asset(src/foo.ts)" ]
      //     4 [ label = "Asset(src/s.ts)" ]
      //     0 -> 1 [ label = "entry" ]
      //     0 -> 2 [ label = "async" ]
      //     0 -> 3 [ label = "async" ]
      //     0 -> 4 [ label = "shared-bundle" ]
      // }
      //     "#;
      // assert_eq!(dot.trim(), expected_dot.trim());

      println!("dot: \n{}", dot);
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_bundle_graph_request_can_bundle_a_single_file() {
    let project_dir = setup_test_directory("test-bundle-graph-request").unwrap();

    create_dir_all(project_dir.join("src")).unwrap();
    write(
      project_dir.join("src/index.ts"),
      r#"console.log("Hello, world!");"#,
    )
    .unwrap();

    let atlaspack = make_test_atlaspack(&[project_dir.join("src/index.ts")])
      .await
      .unwrap();
    let BundleGraphRequestOutput { bundle_graph, .. } = atlaspack
      .run_request_async(BundleGraphRequest::default())
      .await
      .unwrap()
      .into_bundle_graph()
      .expect("BundleGraphRequest returned a non-BundleGraph result");

    assert_eq!(bundle_graph.num_bundles(), 1);
    let bundle_ref = bundle_graph.bundles().next().unwrap();
    let BundleGraphBundle { bundle, assets } = bundle_ref.bundle_graph_bundle();

    assert_eq!(bundle.bundle_type, FileType::Js);
    assert_eq!(assets.node_count(), 1);
    let asset_ref = assets.node_weights().next().unwrap();
    assert_eq!(asset_ref.file_path(), project_dir.join("src/index.ts"));
    assert_eq!(asset_ref.file_type(), &FileType::Js);
  }

  #[tokio::test(flavor = "multi_thread")]
  #[ignore]
  async fn test_bundle_an_html_file_pointing_to_a_js_file() {
    let project_dir =
      setup_test_directory("test-bundle-an-html-file-pointing-to-a-js-file").unwrap();

    create_dir_all(project_dir.join("src")).unwrap();
    write(
      project_dir.join("src/index.ts"),
      r#"console.log("Hello, world!");"#,
    )
    .unwrap();
    write(
      project_dir.join("src/index.html"),
      r#"
      <script src="index.ts"></script>
      "#,
    )
    .unwrap();

    let atlaspack = make_test_atlaspack(&[project_dir.join("src/index.html")])
      .await
      .unwrap();
    let BundleGraphRequestOutput { bundle_graph, .. } = atlaspack
      .run_request_async(BundleGraphRequest::default())
      .await
      .unwrap()
      .into_bundle_graph()
      .expect("BundleGraphRequest returned a non-BundleGraph result");

    assert_eq!(bundle_graph.num_bundles(), 2);
    let html_bundle = bundle_graph
      .bundles()
      .find(|bundle| bundle.bundle_type() == &FileType::Html)
      .unwrap();

    let js_bundle = bundle_graph
      .bundles()
      .find(|bundle| bundle.bundle_type() == &FileType::Js)
      .unwrap();

    assert_eq!(html_bundle.num_assets(), 1);
    assert_eq!(js_bundle.num_assets(), 1);

    let html = html_bundle.assets().next().unwrap();
    let js = js_bundle.assets().next().unwrap();

    assert_eq!(html.file_path(), project_dir.join("src/index.html"));
    assert_eq!(js.file_path(), project_dir.join("src/index.ts"));

    let referenced_bundles = bundle_graph.referenced_bundles(&html_bundle);
    assert_eq!(referenced_bundles.len(), 1);
    assert_eq!(
      referenced_bundles[0].bundle_graph_bundle().bundle,
      js_bundle.bundle_graph_bundle().bundle
    );
  }
}
