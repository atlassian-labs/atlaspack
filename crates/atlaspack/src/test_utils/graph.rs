use std::collections::HashMap;

use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};

pub fn map_graph<N, E, ON, OE>(
  graph: &StableDiGraph<N, E>,
  map_node: impl Fn(&N) -> ON,
  map_edge: impl Fn(&E) -> OE,
) -> StableDiGraph<ON, OE> {
  let mut result = StableDiGraph::new();
  let mut node_index_rewrites = HashMap::new();

  for (node_index, node_weight) in graph.node_references() {
    let mapped_node = map_node(node_weight);
    let mapped_node_index = result.add_node(mapped_node);
    node_index_rewrites.insert(node_index, mapped_node_index);
  }

  for edge_reference in graph.edge_references() {
    let edge_weight = edge_reference.weight();
    let mapped_edge = map_edge(edge_weight);

    result.add_edge(
      node_index_rewrites[&edge_reference.source()],
      node_index_rewrites[&edge_reference.target()],
      mapped_edge,
    );
  }

  result
}

pub fn find_node<N, E>(
  graph: &StableDiGraph<N, E>,
  predicate: impl Fn(&N) -> bool,
) -> Option<(NodeIndex, &N)> {
  graph.node_references().find(|(_, node)| predicate(node))
}

pub fn expect_node<N, E>(
  graph: &StableDiGraph<N, E>,
  predicate: impl Fn(&N) -> bool,
) -> (NodeIndex, &N) {
  find_node(graph, predicate).unwrap()
}

pub fn expect_edge<'a, N: std::fmt::Debug, E>(
  graph: &'a StableDiGraph<N, E>,
  source: NodeIndex,
  target: NodeIndex,
) -> petgraph::stable_graph::EdgeReference<'a, E> {
  let source_weight = graph.node_weight(source).unwrap();
  let target_weight = graph.node_weight(target).unwrap();

  graph
    .edges_connecting(source, target)
    .next()
    .expect(&format!(
      "Expected edge between source and target: {source_weight:#?} \n  -----> {target_weight:#?}"
    ))
}

#[cfg(test)]
mod test {
  use petgraph::graph::NodeIndex;

  use super::*;

  #[test]
  fn test_map_graph() {
    let mut graph = StableDiGraph::<usize, usize>::new();

    let a = graph.add_node(1);
    let b = graph.add_node(2);
    let c = graph.add_node(3);
    let d = graph.add_node(4);

    graph.add_edge(a, b, 1);
    graph.add_edge(a, c, 2);
    graph.add_edge(c, d, 3);

    let mapped_graph = map_graph(&graph, |node| node.to_string(), |edge| edge.to_string());

    assert_eq!(mapped_graph.node_count(), 4);
    assert_eq!(mapped_graph.edge_count(), 3);

    assert_eq!(mapped_graph.node_weight(a).unwrap(), "1");
    assert_eq!(mapped_graph.node_weight(b).unwrap(), "2");
    assert_eq!(mapped_graph.node_weight(c).unwrap(), "3");
    assert_eq!(mapped_graph.node_weight(d).unwrap(), "4");

    let get_edge =
      |a: NodeIndex, b: NodeIndex| mapped_graph.edges_connecting(a, b).next().unwrap().weight();

    assert_eq!(get_edge(a, b), "1");
    assert_eq!(get_edge(a, c), "2");
    assert_eq!(get_edge(c, d), "3");
  }

  #[test]
  fn test_expect_node() {
    let mut graph = StableDiGraph::<usize, usize>::new();

    let a = graph.add_node(1);
    let b = graph.add_node(2);
    let c = graph.add_node(3);

    assert_eq!(expect_node(&graph, |node| *node == 1), (a, &1));
    assert_eq!(expect_node(&graph, |node| *node == 2), (b, &2));
    assert_eq!(expect_node(&graph, |node| *node == 3), (c, &3));
  }

  #[test]
  fn test_find_node() {
    let mut graph = StableDiGraph::<usize, usize>::new();

    let a = graph.add_node(1);
    let b = graph.add_node(2);
    let c = graph.add_node(3);

    assert_eq!(find_node(&graph, |node| *node == 1), Some((a, &1)));
    assert_eq!(find_node(&graph, |node| *node == 2), Some((b, &2)));
    assert_eq!(find_node(&graph, |node| *node == 3), Some((c, &3)));
    assert_eq!(find_node(&graph, |node| *node == 4), None);
  }
}
