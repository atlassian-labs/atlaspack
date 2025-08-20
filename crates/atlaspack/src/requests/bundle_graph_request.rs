use std::collections::HashMap;

use async_trait::async_trait;
use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, AssetNode, DependencyNode},
  bundle_graph::{BundleGraph, BundleGraphBundle},
  types::{Bundle, BundleBehavior, Environment, Priority, Target},
};
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
  Direction,
};
use tracing::{debug, info};

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::{AssetGraphRequest, RequestResult},
};

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

    let simplified_graph = simplify_graph(&asset_graph.graph);
    let root_node = NodeIndex::new(0);
    assert_eq!(
      simplified_graph.node_weight(root_node).unwrap(),
      &SimplifiedAssetGraphNode::Root
    );

    let acyclic_graph = remove_cycles(&simplified_graph);
    let root_node = acyclic_graph
      .node_references()
      .find(|(_, node)| matches!(node, AcyclicAssetGraphNode::Root))
      .unwrap()
      .0;
    let dominator_tree = build_dominator_tree(&acyclic_graph, root_node);

    let mut bundle_graph = BundleGraph::default();

    for edge in dominator_tree.edges(root_node) {
      let start_node = edge.target();
      let mut bundle_assets = StableDiGraph::new();
      let mut bundle_asset_node_index_by_id = HashMap::new();

      enum Control {
        Continue,
        Break,
        Prune,
      }

      impl petgraph::visit::ControlFlow for Control {
        fn continuing() -> Self {
          Control::Continue
        }
        fn should_break(&self) -> bool {
          matches!(self, Control::Break)
        }
        fn should_prune(&self) -> bool {
          matches!(self, Control::Prune)
        }
      }

      petgraph::visit::depth_first_search(&dominator_tree, [start_node], |event| match event {
        petgraph::visit::DfsEvent::Discover(node_index, _) => {
          let node_weight = dominator_tree.node_weight(node_index).unwrap();
          match node_weight {
            AcyclicAssetGraphNode::Asset(asset_node) => {
              let node_index = bundle_assets.add_node(*asset_node.asset.id());
              bundle_asset_node_index_by_id.insert(*asset_node.asset.id(), node_index);

              Control::Continue
            }
            AcyclicAssetGraphNode::Cycle(assets) => {
              for asset in assets {
                let node_index = bundle_assets.add_node(*asset.asset.id());
                bundle_asset_node_index_by_id.insert(*asset.asset.id(), node_index);
              }

              Control::Continue
            }
            _ => Control::Continue,
          }
        }
        _ => Control::Continue,
      });

      fn get_asset_ids(node_weight: &AcyclicAssetGraphNode) -> Vec<u64> {
        match node_weight {
          AcyclicAssetGraphNode::Asset(asset_node) => vec![*asset_node.asset.id()],
          AcyclicAssetGraphNode::Cycle(assets) => {
            assets.iter().map(|asset| *asset.asset.id()).collect()
          }
          AcyclicAssetGraphNode::Root => vec![],
          AcyclicAssetGraphNode::Entry => vec![],
          AcyclicAssetGraphNode::None => vec![],
        }
      }

      for edge in dominator_tree.edges(start_node) {
        let source_node_index = edge.source();
        let target_node_index = edge.target();
        let source_node_weight = dominator_tree.node_weight(source_node_index).unwrap();
        let target_node_weight = dominator_tree.node_weight(target_node_index).unwrap();
        let source_asset_ids = get_asset_ids(source_node_weight);
        let target_asset_ids = get_asset_ids(target_node_weight);

        for source_asset_id in &source_asset_ids {
          for target_asset_id in &target_asset_ids {
            let source_bundle_index = bundle_asset_node_index_by_id.get(source_asset_id).unwrap();
            let target_bundle_index = bundle_asset_node_index_by_id.get(target_asset_id).unwrap();

            bundle_assets.add_edge(*source_bundle_index, *target_bundle_index, ());
          }
        }
      }

      let AcyclicAssetGraphNode::Asset(entry_asset) =
        dominator_tree.node_weight(start_node).unwrap()
      else {
        tracing::debug!(?start_node, "Bundle entry node is not an asset");
        continue;
      };

      let file_name = entry_asset.asset.file_path.file_stem().unwrap();
      let bundle_id = format!("bundle(entry={})", entry_asset.asset.id);
      let bundle_name = Some(format!(
        "{}.{}",
        file_name.to_string_lossy(),
        entry_asset.asset.file_type.extension()
      ));

      let bundle_graph_bundle = BundleGraphBundle {
        bundle: Bundle {
          bundle_behavior: Some(BundleBehavior::Isolated),
          bundle_type: entry_asset.asset.file_type.clone(),
          entry_asset_ids: vec![*entry_asset.asset.id()],
          env: Environment::default(),
          hash_reference: "".to_string(),
          id: bundle_id,
          is_splittable: true,
          main_entry_id: Some(*entry_asset.asset.id()),
          manual_shared_bundle: None,
          name: bundle_name,
          needs_stable_name: false,
          pipeline: None,
          public_id: None,
          target: Target::default(),
        },
        assets: bundle_assets,
      };

      bundle_graph.add_bundle(bundle_graph_bundle);
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::BundleGraph(BundleGraphRequestOutput {
        bundle_graph,
        asset_graph: asset_graph.graph,
      }),
      invalidations: vec![],
    })
  }
}

#[derive(Clone, PartialEq)]
pub enum SimplifiedAssetGraphNode {
  Root,
  Entry,
  Asset(AssetNode),
  None,
}

impl std::fmt::Debug for SimplifiedAssetGraphNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SimplifiedAssetGraphNode::Root => write!(f, "Root"),
      SimplifiedAssetGraphNode::Entry => write!(f, "Entry"),
      SimplifiedAssetGraphNode::Asset(asset_node) => {
        write!(f, "Asset({:?})", asset_node.asset.file_path)
      }
      SimplifiedAssetGraphNode::None => {
        write!(f, "None")
      }
    }
  }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SimplifiedAssetGraphEdge {
  /// Root to asset, means the asset is an entry-point
  EntryAsset(DependencyNode),
  /// Root to asset, means the asset is an async import
  AsyncImport(DependencyNode),
  /// Root to asset, means the asset has been split due to type change
  TypeChange(DependencyNode),
  Dependency(DependencyNode),
  AsyncDependency(DependencyNode),
  None,
}

pub type SimplifiedAssetGraph = StableDiGraph<SimplifiedAssetGraphNode, SimplifiedAssetGraphEdge>;

pub fn simplify_graph(graph: &AssetGraph) -> SimplifiedAssetGraph {
  {
    // TODO: Debugging code
    let root_node_index = graph.root_node();
    let root_edges = graph
      .graph
      .edges_directed(root_node_index, Direction::Outgoing);
    assert!(
      root_edges.count() >= 1,
      "Root node must have at least one outgoing edge"
    );
  }

  let mut simplified_graph = StableDiGraph::new();
  let mut root_node_index = None;

  let mut none_indexes = Vec::new();
  for node in graph.nodes() {
    match node {
      AssetGraphNode::Root => {
        root_node_index = Some(simplified_graph.add_node(SimplifiedAssetGraphNode::Root));
      }
      AssetGraphNode::Entry => {
        simplified_graph.add_node(SimplifiedAssetGraphNode::Entry);
      }
      AssetGraphNode::Asset(asset_node) => {
        simplified_graph.add_node(SimplifiedAssetGraphNode::Asset(asset_node.clone()));
      }
      AssetGraphNode::Dependency(_dependency_node) => {
        let node_index = simplified_graph.add_node(SimplifiedAssetGraphNode::None);
        none_indexes.push(node_index);
      }
    };
  }
  // TODO: Do not crash if there is no root node
  let root_node_index = root_node_index.unwrap();

  for edge in graph.graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();
    let source_node = graph.get_node(&source_node_index).unwrap();
    let target_node = graph.get_node(&target_node_index).unwrap();

    if let AssetGraphNode::Dependency(dependency_node) = source_node {
      assert!(matches!(target_node, AssetGraphNode::Asset(_)));

      for incoming_edge in graph
        .graph
        .edges_directed(source_node_index, Direction::Incoming)
      {
        let incoming_node_index = incoming_edge.source();
        let target_node = graph.get_node(&target_node_index).unwrap();

        assert!(
          matches!(target_node, AssetGraphNode::Asset(_))
            || matches!(target_node, AssetGraphNode::Root),
          "Target node must be an asset or root"
        );

        // The input graph looks like:
        //
        // a -> dependency_a_to_b -> b
        // a -> dependency_a_to_c -> c
        //
        // We are rewriting it into:
        //
        // a -> b
        // a -> c
        //
        // And storing the dependency as an edge weight.
        if let (
          Some(AssetGraphNode::Asset(source_asset_node)),
          AssetGraphNode::Asset(target_asset_node),
        ) = (graph.get_node(&incoming_node_index), target_node)
        {
          if source_asset_node.asset.file_type != target_asset_node.asset.file_type {
            debug!(
              "Skipping dependency edge because source and target have different file types: {:?} -> {:?}",
              source_asset_node.asset.file_path, target_asset_node.asset.file_path
            );
            simplified_graph.add_edge(
              root_node_index,
              target_node_index,
              SimplifiedAssetGraphEdge::TypeChange(dependency_node.clone()),
            );
            // TODO: We should still mark this on the simplified graph
            // but the edge types should be different so we can ignore on certain traversals.
            continue;
          }
        }

        if dependency_node.dependency.priority != Priority::Sync {
          simplified_graph.add_edge(
            root_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::AsyncImport(dependency_node.clone()),
          );
          continue;
        }

        if incoming_node_index == root_node_index {
          simplified_graph.add_edge(
            incoming_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::EntryAsset(dependency_node.clone()),
          );
        } else {
          simplified_graph.add_edge(
            incoming_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::Dependency(dependency_node.clone()),
          );
        }
      }
    } else {
      // println!(
      //   "Skipping edge because source-node is not a dependency: {:#?} -> {:#?}",
      //   source_node, target_node
      // );
      // panic!(
      //   "Invalid graph ; source-node: {:#?}, target-node: {:#?}",
      //   source_node, target_node
      // );

      // simplified_graph.add_edge(
      //   source_node_index,
      //   target_node_index,
      //   SimplifiedAssetGraphEdge::None,
      // );
    }
  }

  for node_index in none_indexes {
    simplified_graph.remove_node(node_index);
  }

  simplified_graph
}

#[derive(Clone, PartialEq, Debug)]
pub enum AcyclicAssetGraphNode {
  Root,
  Entry,
  Asset(AssetNode),
  Cycle(Vec<AssetNode>),
  None,
}

impl From<SimplifiedAssetGraphNode> for AcyclicAssetGraphNode {
  fn from(node: SimplifiedAssetGraphNode) -> Self {
    match node {
      SimplifiedAssetGraphNode::Root => AcyclicAssetGraphNode::Root,
      SimplifiedAssetGraphNode::Entry => AcyclicAssetGraphNode::Entry,
      SimplifiedAssetGraphNode::Asset(asset_node) => AcyclicAssetGraphNode::Asset(asset_node),
      SimplifiedAssetGraphNode::None => AcyclicAssetGraphNode::None,
    }
  }
}

pub type AcyclicAssetGraph = StableDiGraph<AcyclicAssetGraphNode, SimplifiedAssetGraphEdge>;

pub fn remove_cycles(graph: &SimplifiedAssetGraph) -> AcyclicAssetGraph {
  let mut result = StableDiGraph::new();
  let sccs = petgraph::algo::tarjan_scc(graph);
  let mut index_map = HashMap::new();

  for scc in sccs {
    if scc.len() == 1 {
      let node_index = scc[0];
      let node_weight = graph.node_weight(node_index).unwrap();
      let new_weight: AcyclicAssetGraphNode = AcyclicAssetGraphNode::from(node_weight.clone());
      let new_index = result.add_node(new_weight);
      index_map.insert(node_index, new_index);
    } else {
      let assets = scc
        .iter()
        .map(|node_index| {
          let node_weight = graph.node_weight(*node_index).unwrap();
          match node_weight {
            SimplifiedAssetGraphNode::Asset(asset_node) => asset_node.clone(),
            _ => unreachable!(),
          }
        })
        .collect();

      let new_weight = AcyclicAssetGraphNode::Cycle(assets);
      let new_index = result.add_node(new_weight);
      for node_index in scc {
        index_map.insert(node_index, new_index);
      }
    }
  }

  for edge in graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();

    let new_source_node_index = index_map.get(&source_node_index).unwrap();
    let new_target_node_index = index_map.get(&target_node_index).unwrap();
    if new_source_node_index == new_target_node_index {
      continue;
    }

    result.add_edge(
      *new_source_node_index,
      *new_target_node_index,
      edge.weight().clone(),
    );
  }

  result
}

type DominatorTreeNode = AcyclicAssetGraphNode;

enum DominatorTreeEdge {
  /// From root to asset, means the asset is an entry-point
  EntryAsset(DependencyNode),
  /// From root to asset, means the asset is an async import
  AsyncImport(DependencyNode),
  /// From root to asset, means the asset is a type change
  TypeChange(DependencyNode),
  /// From root to asset, means the asset is a shared bundle
  SharedBundle,
  /// From asset to asset, means the asset is a dependency of the other within a bundle
  Dependency(DependencyNode),
  /// From asset to asset, means the asset is an async dependency of the other within a bundle
  AsyncDependency(DependencyNode),
}

pub fn build_dominator_tree(
  graph: &AcyclicAssetGraph,
  root_id: NodeIndex,
) -> StableDiGraph<DominatorTreeNode, DominatorTreeEdge> {
  let dominators = petgraph::algo::dominators::simple_fast(graph, root_id);
  let mut result = StableDiGraph::new();

  let mut root_node_index: Option<NodeIndex> = None;
  for node_index in graph.node_indices() {
    let node_weight = graph.node_weight(node_index).unwrap();
    let node_index = result.add_node(node_weight.clone());
    if node_weight == &AcyclicAssetGraphNode::Root {
      root_node_index = Some(node_index);
    }
  }
  let root_node_index = root_node_index.unwrap();

  for node_index in graph.node_indices() {
    let immediate_dominator = dominators.immediate_dominator(node_index);

    if let Some(dominator_index) = immediate_dominator {
      let edges = graph.edges_connecting(dominator_index, node_index);

      let dominator_weight = graph.node_weight(dominator_index).unwrap();
      let node_weight = graph.node_weight(node_index).unwrap();

      // println!("node: {:?}", node_weight);
      // println!("dominator: {:?}", dominator_weight);

      if !graph.contains_edge(dominator_index, node_index) {
        result.add_edge(dominator_index, node_index, DominatorTreeEdge::SharedBundle);
        assert!(dominator_index == root_node_index);
      } else {
        for edge in edges {
          let weight = edge.weight();
          let weight = match weight {
            SimplifiedAssetGraphEdge::EntryAsset(dependency_node) => {
              DominatorTreeEdge::EntryAsset(dependency_node.clone())
            }
            SimplifiedAssetGraphEdge::AsyncImport(dependency_node) => {
              DominatorTreeEdge::AsyncImport(dependency_node.clone())
            }
            SimplifiedAssetGraphEdge::TypeChange(dependency_node) => {
              DominatorTreeEdge::TypeChange(dependency_node.clone())
            }
            SimplifiedAssetGraphEdge::Dependency(dependency_node) => {
              DominatorTreeEdge::Dependency(dependency_node.clone())
            }
            SimplifiedAssetGraphEdge::AsyncDependency(dependency_node) => {
              DominatorTreeEdge::AsyncDependency(dependency_node.clone())
            }
            SimplifiedAssetGraphEdge::None => panic!("Invalid graph"),
          };
          result.add_edge(dominator_index, node_index, weight);
        }
      }
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use std::{
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
  };

  use atlaspack_core::types::Asset;
  use petgraph::{
    algo::toposort,
    visit::{IntoEdgesDirected, IntoNodeReferences},
  };

  use crate::test_utils::{
    get_repo_path, graph::expect_node, make_test_atlaspack, setup_test_directory,
  };

  use super::*;

  fn expect_asset<'a>(
    dominator_tree: &'a StableDiGraph<DominatorTreeNode, DominatorTreeEdge>,
    project_dir: &Path,
    path: &str,
  ) -> (NodeIndex, &'a DominatorTreeNode) {
    let predicate = move |node: &DominatorTreeNode| -> bool {
      let DominatorTreeNode::Asset(AssetNode {
        asset: Asset { file_path, .. },
        ..
      }) = node
      else {
        return false;
      };

      file_path == &project_dir.join(path)
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
      SimplifiedAssetGraphNode::Entry => "Entry".to_string(),
      SimplifiedAssetGraphNode::None => "None".to_string(),
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
      SimplifiedAssetGraphEdge::Dependency(_) => "sync".to_string(),
      SimplifiedAssetGraphEdge::AsyncImport(_) => "async".to_string(),
      SimplifiedAssetGraphEdge::TypeChange(_) => "type-change".to_string(),
      SimplifiedAssetGraphEdge::EntryAsset(_) => "entry".to_string(),
      SimplifiedAssetGraphEdge::AsyncDependency(_) => "async".to_string(),
      SimplifiedAssetGraphEdge::None => "none".to_string(),
    }
  }

  fn dominator_tree_edge_label(edge: &DominatorTreeEdge) -> String {
    match edge {
      DominatorTreeEdge::EntryAsset(_) => "entry".to_string(),
      DominatorTreeEdge::AsyncImport(_) => "async".to_string(),
      DominatorTreeEdge::TypeChange(_) => "type-change".to_string(),
      DominatorTreeEdge::SharedBundle => "shared-bundle".to_string(),
      DominatorTreeEdge::Dependency(_) => "sync".to_string(),
      DominatorTreeEdge::AsyncDependency(_) => "async".to_string(),
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
      |node| !matches!(node, SimplifiedAssetGraphNode::None),
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
      AcyclicAssetGraphNode::Entry => "Entry".to_string(),
      AcyclicAssetGraphNode::None => "None".to_string(),
    }
  }

  fn make_acyclic_dot_graph(project_dir: PathBuf, acyclic_graph: AcyclicAssetGraph) -> DebugGraph {
    let repo_root = get_repo_path();

    make_dot_graph(
      &acyclic_graph,
      |node| acyclic_asset_graph_node_label(node, &repo_root, &project_dir),
      |edge| simplified_asset_graph_edge_label(edge),
      |node| !matches!(node, AcyclicAssetGraphNode::None),
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
        AcyclicAssetGraphNode::Entry => "Entry".to_string(),
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
        AcyclicAssetGraphNode::None => "None".to_string(),
      },
      |edge| dominator_tree_edge_label(edge),
      |node| !matches!(node, AcyclicAssetGraphNode::None),
    )
  }

  #[tokio::test(flavor = "multi_thread")]
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
    let acyclic_graph = remove_cycles(&simplified_graph);

    let dot_graph = make_acyclic_dot_graph(project_dir, acyclic_graph);
    let dot = petgraph::dot::Dot::new(&dot_graph);
    let dot = format!("{}", dot);
    let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    1 [ label = "Asset(src/baz.ts)" ]
    2 [ label = "Cycle(src/bar.ts, src/foo.ts)" ]
    0 -> 1 [ label = "entry" ]
    1 -> 2 [ label = "sync" ]
    1 -> 2 [ label = "sync" ]
}
    "#;

    println!("dot:\n{}\n\n", dot);

    assert_eq!(dot.trim(), expected_dot.trim());
  }

  mod test_dominator_tree {
    use atlaspack_core::types::Asset;

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

      let acyclic_graph = remove_cycles(&simplified_graph);
      let root_node = acyclic_graph
        .node_references()
        .find(|(_, node)| matches!(node, AcyclicAssetGraphNode::Root))
        .unwrap()
        .0;
      let dominator_tree = build_dominator_tree(&acyclic_graph, root_node);

      dominator_tree
    }

    #[tokio::test(flavor = "multi_thread")]
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
    0 -> 1 [ label = "entry" ]
}
    "#;

      println!("dot: \n{}", dot);
      assert_eq!(dot.trim(), expected_dot.trim());
    }

    #[tokio::test(flavor = "multi_thread")]
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
    0 -> 1 [ label = "entry" ]
    1 -> 2 [ label = "sync" ]
}
    "#;

      println!("dot: \n{}", dot);
      assert_eq!(dot.trim(), expected_dot.trim());
    }

    #[tokio::test(flavor = "multi_thread")]
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
    0 -> 1 [ label = "entry" ]
    0 -> 2 [ label = "async" ]
}
    "#;

      println!("dot: \n{}", dot);
      assert_eq!(dot.trim(), expected_dot.trim());
    }

    #[tokio::test(flavor = "multi_thread")]
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
      assert_eq!(dominator_tree.edge_count(), 5);

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
  }
}
