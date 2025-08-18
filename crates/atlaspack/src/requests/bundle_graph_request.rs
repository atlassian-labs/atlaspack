use std::collections::HashMap;

use async_trait::async_trait;
use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, AssetNode, DependencyNode},
  bundle_graph::{BundleGraph, BundleGraphBundle},
  types::{Bundle, BundleBehavior, Environment, FileType, Target},
};
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
  Direction,
};
use tracing::info;

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
      let mut bundle_assets = vec![];

      petgraph::visit::depth_first_search(&dominator_tree, [start_node], |event| match event {
        petgraph::visit::DfsEvent::Discover(node_index, _) => {
          let node_weight = dominator_tree.node_weight(node_index).unwrap();
          match node_weight {
            AcyclicAssetGraphNode::Asset(asset_node) => {
              bundle_assets.push(asset_node.asset.id.clone());
            }
            AcyclicAssetGraphNode::Cycle(assets) => {
              bundle_assets.extend(assets.iter().map(|asset| asset.asset.id.clone()));
            }
            _ => {}
          }
        }
        petgraph::visit::DfsEvent::TreeEdge(_, _) => {}
        petgraph::visit::DfsEvent::BackEdge(_, _) => {}
        petgraph::visit::DfsEvent::CrossForwardEdge(_, _) => {}
        petgraph::visit::DfsEvent::Finish(_, _) => {}
      });

      let AcyclicAssetGraphNode::Asset(entry_asset) =
        dominator_tree.node_weight(start_node).unwrap()
      else {
        anyhow::bail!("Bundle entry node is not an asset");
      };

      let file_name = entry_asset.asset.file_path.file_stem().unwrap();
      let bundle_id = format!("bundle_{}", file_name.to_string_lossy());
      let bundle_name = Some(format!(
        "{}.{}",
        file_name.to_string_lossy(),
        entry_asset.asset.file_type.extension()
      ));

      let bundle_graph_bundle = BundleGraphBundle {
        bundle: Bundle {
          bundle_behavior: Some(BundleBehavior::Isolated),
          bundle_type: entry_asset.asset.file_type.clone(),
          entry_asset_ids: vec![entry_asset.asset.id.clone()],
          env: Environment::default(),
          hash_reference: "".to_string(),
          id: bundle_id,
          is_splittable: true,
          main_entry_id: Some(entry_asset.asset.id.clone()),
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

#[derive(Clone, PartialEq)]
pub enum SimplifiedAssetGraphEdge {
  Dependency(DependencyNode),
  None,
}

impl std::fmt::Debug for SimplifiedAssetGraphEdge {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SimplifiedAssetGraphEdge::Dependency(_dependency_node) => {
        write!(f, "Dependency")
      }
      SimplifiedAssetGraphEdge::None => {
        write!(f, "None")
      }
    }
  }
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
        simplified_graph.add_node(SimplifiedAssetGraphNode::None);
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
            info!(
              "Skipping dependency edge because source and target have different file types: {:?} -> {:?}",
              source_asset_node.asset.file_path, target_asset_node.asset.file_path
            );
            simplified_graph.add_edge(
              root_node_index,
              target_node_index,
              SimplifiedAssetGraphEdge::None,
            );
            // TODO: We should still mark this on the simplified graph
            // but the edge types should be different so we can ignore on certain traversals.
            continue;
          }
        }

        simplified_graph.add_edge(
          incoming_node_index,
          target_node_index,
          SimplifiedAssetGraphEdge::Dependency(dependency_node.clone()),
        );
      }
    } else {
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

pub fn build_dominator_tree(
  graph: &AcyclicAssetGraph,
  root_id: NodeIndex,
) -> StableDiGraph<AcyclicAssetGraphNode, ()> {
  let dominators = petgraph::algo::dominators::simple_fast(graph, root_id);
  let mut result = StableDiGraph::new();

  for node_index in graph.node_indices() {
    let node_weight = graph.node_weight(node_index).unwrap();
    result.add_node(node_weight.clone());
  }

  for node_index in graph.node_indices() {
    let immediate_dominator = dominators.immediate_dominator(node_index);
    if let Some(dominator_index) = immediate_dominator {
      result.add_edge(dominator_index, node_index, ());
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

  use petgraph::visit::IntoNodeReferences;

  use crate::test_utils::{get_repo_path, make_test_atlaspack, setup_test_directory};

  use super::*;

  fn asset_node_label(asset_node: &AssetNode, repo_root: &Path, project_dir: &Path) -> String {
    let file_path = asset_node.asset.file_path.clone();
    let path = if file_path.starts_with(repo_root) {
      PathBuf::from("<atlaspack_repo_root>").join(file_path.strip_prefix(repo_root).unwrap())
    } else if file_path.starts_with(project_dir) {
      file_path.strip_prefix(project_dir).unwrap().to_path_buf()
    } else {
      file_path.clone()
    };

    format!("Asset({:?})", path)
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

    for node in graph.node_weights() {
      let label = node_label_fn(node);
      result.add_node(label);
    }

    for edge in graph.edge_references() {
      let source_node_index = edge.source();
      let target_node_index = edge.target();

      let edge_label = edge_label_fn(edge.weight());
      result.add_edge(source_node_index, target_node_index, edge_label);
    }

    for node_index in graph.node_indices() {
      if !is_node_visible(graph.node_weight(node_index).unwrap()) {
        result.remove_node(node_index);
      }
    }

    result
  }

  type DebugGraph = StableDiGraph<String, String>;

  fn make_simplified_dot_graph(
    project_dir: PathBuf,
    simplified_graph: SimplifiedAssetGraph,
  ) -> DebugGraph {
    let repo_root = get_repo_path();
    make_dot_graph(
      &simplified_graph,
      |node| simplified_asset_graph_node_label(node, &repo_root, &project_dir),
      |_edge| "sync".to_string(),
      |node| !matches!(node, SimplifiedAssetGraphNode::None),
    )
  }

  fn make_acyclic_dot_graph(project_dir: PathBuf, acyclic_graph: AcyclicAssetGraph) -> DebugGraph {
    let repo_root = get_repo_path();

    make_dot_graph(
      &acyclic_graph,
      |node| match node {
        AcyclicAssetGraphNode::Asset(asset_node) => {
          asset_node_label(asset_node, &repo_root, &project_dir)
        }
        AcyclicAssetGraphNode::Cycle(assets) => {
          format!(
            "Cycle({:?})",
            assets
              .iter()
              .map(|asset| asset_node_label(asset, &repo_root, &project_dir))
              .collect::<Vec<_>>()
          )
        }
        AcyclicAssetGraphNode::Root => "Root".to_string(),
        AcyclicAssetGraphNode::Entry => "Entry".to_string(),
        AcyclicAssetGraphNode::None => "None".to_string(),
      },
      |_edge| "sync".to_string(),
      |node| !matches!(node, AcyclicAssetGraphNode::None),
    )
  }

  #[tokio::test]
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
      .run_request(AssetGraphRequest::default())
      .unwrap()
      .into_asset_graph()
      .unwrap();

    let simplified_graph = simplify_graph(&asset_graph.graph);

    let dot_graph = make_simplified_dot_graph(project_dir, simplified_graph);
    let dot = petgraph::dot::Dot::new(&dot_graph);
    let dot = format!("{}", dot);
    let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    2 [ label = "Asset(\"src/index.ts\")" ]
    4 [ label = "Asset(\"src/foo.ts\")" ]
    6 [ label = "Asset(\"<atlaspack_repo_root>/packages/transformers/js/src/esmodule-helpers.js\")" ]
    0 -> 2 [ label = "sync" ]
    2 -> 4 [ label = "sync" ]
    4 -> 6 [ label = "sync" ]
}
    "#;

    println!("dot:\n{}\n\n", dot);
    assert_eq!(dot.trim(), expected_dot.trim());
  }

  #[tokio::test]
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
      .run_request(AssetGraphRequest::default())
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
    2 [ label = "Asset(\"<atlaspack_repo_root>/packages/transformers/js/src/esmodule-helpers.js\")" ]
    3 [ label = "Cycle([\"Asset(\\\"src/foo.ts\\\")\", \"Asset(\\\"src/bar.ts\\\")\"])" ]
    4 [ label = "Asset(\"src/baz.ts\")" ]
    4 -> 3 [ label = "sync" ]
    4 -> 3 [ label = "sync" ]
    3 -> 2 [ label = "sync" ]
    3 -> 2 [ label = "sync" ]
    4 -> 2 [ label = "sync" ]
}
    "#;

    println!("dot:\n{}\n\n", dot);

    // assert_eq!(dot.trim(), expected_dot.trim());
  }

  #[tokio::test]
  async fn test_build_dominator_tree() {
    let project_dir = setup_test_directory("test_build_dominator_tree").unwrap();
    create_dir_all(project_dir.join("src")).unwrap();
    write(
      project_dir.join("src/foo.ts"),
      r#"export const foo = "foo";"#,
    )
    .unwrap();

    let atlaspack = make_test_atlaspack(&[project_dir.join("src/foo.ts")])
      .await
      .unwrap();
    let asset_graph = atlaspack
      .run_request(AssetGraphRequest::default())
      .unwrap()
      .into_asset_graph()
      .unwrap();
    let simplified_graph = simplify_graph(&asset_graph.graph);
    let acyclic_graph = remove_cycles(&simplified_graph);
    let root_node = acyclic_graph
      .node_references()
      .find(|(_, node)| matches!(node, AcyclicAssetGraphNode::Root))
      .unwrap()
      .0;
    let dominator_tree = build_dominator_tree(&acyclic_graph, root_node);

    let repo_root = get_repo_path();
    let dot_graph = make_dot_graph(
      &dominator_tree,
      |node| match node {
        AcyclicAssetGraphNode::Root => "Root".to_string(),
        AcyclicAssetGraphNode::Entry => "Entry".to_string(),
        AcyclicAssetGraphNode::Asset(asset_node) => {
          asset_node_label(asset_node, &repo_root, &project_dir)
        }
        AcyclicAssetGraphNode::Cycle(assets) => {
          format!(
            "Cycle({:?})",
            assets
              .iter()
              .map(|asset| asset_node_label(asset, &repo_root, &project_dir))
              .collect::<Vec<_>>()
          )
        }
        AcyclicAssetGraphNode::None => "None".to_string(),
      },
      |_edge| "sync".to_string(),
      |node| !matches!(node, AcyclicAssetGraphNode::None),
    );

    let dot = petgraph::dot::Dot::new(&dot_graph);
    let dot = format!("{}", dot);
    let expected_dot = r#"
digraph {
    0 [ label = "Root" ]
    2 [ label = "Asset(\"src/foo.ts\")" ]
    3 [ label = "Asset(\"<atlaspack_repo_root>/packages/transformers/js/src/esmodule-helpers.js\")" ]
    0 -> 2 [ label = "sync" ]
    2 -> 3 [ label = "sync" ]
}
    "#;

    println!("dot: \n{}", dot);
    assert_eq!(dot.trim(), expected_dot.trim());
  }

  #[tokio::test]
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
      .run_request(BundleGraphRequest::default())
      .unwrap()
      .into_bundle_graph()
      .expect("BundleGraphRequest returned a non-BundleGraph result");

    assert_eq!(bundle_graph.num_bundles(), 1);
  }
}
