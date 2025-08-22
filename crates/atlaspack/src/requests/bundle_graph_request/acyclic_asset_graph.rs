use std::collections::HashMap;

use atlaspack_core::bundle_graph::AssetRef;
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences},
};

use super::simplified_graph::*;

#[derive(Clone, PartialEq, Debug)]
pub enum AcyclicAssetGraphNode {
  Root,
  Asset(AssetRef),
  Cycle(Vec<AssetRef>),
}

impl AcyclicAssetGraphNode {
  pub fn file_path(&self) -> Option<&std::path::Path> {
    match self {
      AcyclicAssetGraphNode::Asset(asset_ref) => Some(asset_ref.file_path()),
      _ => None,
    }
  }
}

impl From<SimplifiedAssetGraphNode> for AcyclicAssetGraphNode {
  fn from(node: SimplifiedAssetGraphNode) -> Self {
    match node {
      SimplifiedAssetGraphNode::Root => AcyclicAssetGraphNode::Root,
      SimplifiedAssetGraphNode::Asset(asset_ref) => AcyclicAssetGraphNode::Asset(asset_ref),
    }
  }
}

pub type AcyclicAssetGraph = StableDiGraph<AcyclicAssetGraphNode, SimplifiedAssetGraphEdge>;

pub fn remove_cycles(graph: &SimplifiedAssetGraph) -> (NodeIndex, AcyclicAssetGraph) {
  let mut result: StableDiGraph<Option<AcyclicAssetGraphNode>, SimplifiedAssetGraphEdge> =
    graph.map(|_, _| None, |_, edge| edge.clone());

  let sccs = petgraph::algo::tarjan_scc(graph);
  let mut root_node_index: Option<NodeIndex> = None;
  let mut index_map = HashMap::new();

  for scc in &sccs {
    if scc.len() == 1 {
      let node_index = scc[0];
      let node_weight = graph.node_weight(node_index).unwrap();
      let new_weight: AcyclicAssetGraphNode = AcyclicAssetGraphNode::from(node_weight.clone());
      let weight = result.node_weight_mut(node_index).unwrap();
      *weight = Some(new_weight);

      if node_weight == &SimplifiedAssetGraphNode::Root {
        root_node_index = Some(node_index);
      }
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
      let new_index = result.add_node(Some(new_weight));

      for node_index in scc {
        index_map.insert(node_index, new_index);
      }
    }
  }

  for edge in graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();

    // Being in the map means these
    let new_source_node_index = index_map.get(&source_node_index);
    let new_target_node_index = index_map.get(&target_node_index);

    let either_node_is_in_cycle =
      new_source_node_index.is_some() || new_target_node_index.is_some();
    if !either_node_is_in_cycle {
      continue;
    }

    let new_source_node_index = new_source_node_index.unwrap_or(&source_node_index);
    let new_target_node_index = new_target_node_index.unwrap_or(&target_node_index);

    if new_source_node_index == new_target_node_index {
      continue;
    }

    result.add_edge(
      *new_source_node_index,
      *new_target_node_index,
      edge.weight().clone(),
    );
  }

  for scc in &sccs {
    if scc.len() == 1 {
      continue;
    }

    for node_index in scc {
      result.remove_node(*node_index);
    }
  }

  let result = result.filter_map(
    |_, node_weight| node_weight.clone(),
    |_, edge| Some(edge.clone()),
  );

  (
    root_node_index.expect("The root can't cycle onto assets. Something is wrong"),
    result,
  )
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use petgraph::{graph::NodeIndex, visit::IntoNodeReferences};

  use crate::test_utils::graph::expect_edge;

  use super::super::asset_graph_builder::simplified_asset_graph_builder;
  use super::*;

  fn find_root<N, E>(graph: &StableDiGraph<N, E>) -> NodeIndex {
    graph
      .node_references()
      .find_map(|(idx, _)| match idx.index() {
        _ => Some(idx),
      })
      .expect("graph should have nodes")
  }

  fn find_root_simplified(graph: &SimplifiedAssetGraph) -> NodeIndex {
    graph
      .node_references()
      .find_map(|(idx, node)| match node {
        SimplifiedAssetGraphNode::Root => Some(idx),
        _ => None,
      })
      .expect("root not found")
  }

  fn find_cycle_node(graph: &AcyclicAssetGraph) -> (NodeIndex, Vec<PathBuf>) {
    for (idx, node) in graph.node_references() {
      if let AcyclicAssetGraphNode::Cycle(assets) = node {
        let mut files = assets
          .iter()
          .map(|a| a.file_path().to_path_buf())
          .collect::<Vec<_>>();
        files.sort();
        return (idx, files);
      }
    }
    panic!("cycle node not found")
  }

  #[test]
  fn test_remove_cycles_identity_for_acyclic_graph() {
    // dot:
    // digraph {
    //   root -> a
    //   a -> c
    // }
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let c = graph.asset("src/c.ts");
    graph.sync_dependency(a, c);
    let graph = graph.build();

    let original_root = find_root_simplified(&graph);
    let (root, acyclic) = remove_cycles(&graph);

    assert_eq!(root, original_root);
    assert_eq!(acyclic.node_count(), 3);
    assert_eq!(acyclic.edge_count(), 2);

    assert!(matches!(
      acyclic.node_weight(root).unwrap(),
      AcyclicAssetGraphNode::Root
    ));

    assert!(matches!(
      expect_edge(&acyclic, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&acyclic, a, c).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
  }

  #[test]
  fn test_collapses_two_node_cycle_into_single_cycle_node() {
    // dot:
    // digraph {
    //   root -> a
    //   a -> b
    //   b -> a
    // }
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let b = graph.asset("src/b.ts");
    // Create a cycle between a and b
    graph.sync_dependency(a, b);
    graph.sync_dependency(b, a);
    let graph = graph.build();

    let (root, acyclic) = remove_cycles(&graph);

    assert_eq!(acyclic.node_count(), 2);
    assert_eq!(acyclic.edge_count(), 1);

    let (cycle_idx, files) = find_cycle_node(&acyclic);
    assert_eq!(
      files,
      vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")]
    );

    assert!(matches!(
      expect_edge(&acyclic, root, cycle_idx).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
  }

  #[test]
  fn test_edges_out_of_cycle_are_rewired() {
    // dot:
    // digraph {
    //   root -> a
    //   a -> b
    //   b -> a
    //   b -> c
    // }
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let b = graph.asset("src/b.ts");
    let c = graph.asset("src/c.ts");
    // Cycle a <-> b
    graph.sync_dependency(a, b);
    graph.sync_dependency(b, a);
    // Outgoing from cycle to c
    graph.sync_dependency(b, c);
    let graph = graph.build();

    let (root, acyclic) = remove_cycles(&graph);

    assert_eq!(acyclic.node_count(), 3);

    let (cycle_idx, _files) = find_cycle_node(&acyclic);

    assert!(matches!(
      expect_edge(&acyclic, root, cycle_idx).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&acyclic, cycle_idx, c).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
  }

  #[test]
  fn test_edges_into_cycle_are_rewired() {
    // dot:
    // digraph {
    //   root -> a
    //   a -> b
    //   b -> a
    //   c -> a
    // }
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let b = graph.asset("src/b.ts");
    let c = graph.asset("src/c.ts");
    // Cycle a <-> b
    graph.sync_dependency(a, b);
    graph.sync_dependency(b, a);
    // Incoming from c to cycle only (no edge from cycle to c)
    graph.sync_dependency(c, a);
    let graph = graph.build();

    let (root, acyclic) = remove_cycles(&graph);

    assert_eq!(acyclic.node_count(), 3);

    let (cycle_idx, _files) = find_cycle_node(&acyclic);

    assert!(matches!(
      expect_edge(&acyclic, root, cycle_idx).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&acyclic, c, cycle_idx).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
  }

  #[test]
  fn test_multiple_disjoint_cycles_collapse_independently() {
    // dot:
    // digraph {
    //   root -> a
    //   a -> b
    //   b -> a
    //   b -> c
    //   c -> d
    //   d -> e
    //   e -> d
    // }
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let b = graph.asset("src/b.ts");
    let c = graph.asset("src/c.ts");
    let d = graph.asset("src/d.ts");
    let e = graph.asset("src/e.ts");

    // Cycle1: a <-> b
    graph.sync_dependency(a, b);
    graph.sync_dependency(b, a);
    // Link Cycle1 -> c -> Cycle2
    graph.sync_dependency(b, c);
    // Cycle2: d <-> e
    graph.sync_dependency(d, e);
    graph.sync_dependency(e, d);
    // c -> d
    graph.sync_dependency(c, d);

    let graph = graph.build();

    let (_root, acyclic) = remove_cycles(&graph);

    // Nodes: root, Cycle1, c, Cycle2
    assert_eq!(acyclic.node_count(), 4);
    assert_eq!(acyclic.edge_count(), 3);

    // Find cycles
    let mut cycles = vec![];
    for (idx, node) in acyclic.node_references() {
      if let AcyclicAssetGraphNode::Cycle(assets) = node {
        let mut files = assets
          .iter()
          .map(|a| a.file_path().to_path_buf())
          .collect::<Vec<_>>();
        files.sort();
        cycles.push((idx, files));
      }
    }
    assert_eq!(cycles.len(), 2);

    // Ensure expected file sets
    assert!(
      cycles.iter().any(
        |(_, files)| files.as_slice() == [PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")]
      )
    );
    assert!(
      cycles.iter().any(
        |(_, files)| files.as_slice() == [PathBuf::from("src/d.ts"), PathBuf::from("src/e.ts")]
      )
    );
  }

  #[test]
  fn test_file_path_only_for_asset_nodes() {
    // dot:
    // digraph {
    //   root -> a
    //   a -> b
    //   b -> a
    // }
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("src/a.ts");
    let b = graph.asset("src/b.ts");
    graph.sync_dependency(a, b);
    // Create a cycle
    graph.sync_dependency(b, a);
    let graph = graph.build();

    let (_root, acyclic) = remove_cycles(&graph);

    for (_idx, node) in acyclic.node_references() {
      match node {
        AcyclicAssetGraphNode::Asset(asset_ref) => {
          assert!(
            asset_ref.file_path().is_absolute()
              || asset_ref.file_path().to_string_lossy().contains("src/")
          );
        }
        AcyclicAssetGraphNode::Root | AcyclicAssetGraphNode::Cycle(_) => {
          assert!(node.file_path().is_none());
        }
      }
    }
  }
}
