use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::visit::IntoNodeReferences;
use serde::Serialize;

use crate::types::{Asset, Dependency};

use super::{AssetGraph, AssetGraphNode, DependencyState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDependency {
  pub id: String,
  pub dependency: Dependency,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SerializedAssetGraphNode {
  Root,
  Entry,
  Asset {
    value: Asset,
  },
  Dependency {
    value: SerializedDependency,
    has_deferred: bool,
  },
}

#[derive(Debug, Serialize)]
pub struct SerializedAssetGraphResult {
  pub edges: Vec<u32>,
  pub nodes: Vec<SerializedAssetGraphNode>,
}

pub fn serialize_asset_graph_sorted(
  asset_graph: &AssetGraph,
) -> anyhow::Result<SerializedAssetGraphResult> {
  let mut edges = Vec::<(u32, u32)>::new();
  let mut nodes = Vec::<SerializedAssetGraphNode>::new();

  // Track index reassignment after sorting
  let mut nx_to_index = HashMap::<NodeIndex, u32>::new();

  let mut current_nodes = asset_graph
    .graph
    .node_references()
    .collect::<Vec<(NodeIndex, &AssetGraphNode)>>();
  current_nodes.sort_by(|a, b| a.1.id().cmp(&b.1.id()));

  for (current_node_nx, current_node) in current_nodes {
    nx_to_index.insert(current_node_nx, nodes.len() as u32);
    nodes.push(match current_node {
      AssetGraphNode::Root => SerializedAssetGraphNode::Root,
      AssetGraphNode::Entry => SerializedAssetGraphNode::Entry,
      AssetGraphNode::Asset(asset_node) => SerializedAssetGraphNode::Asset {
        value: asset_node.asset.clone(),
      },
      AssetGraphNode::Dependency(dependency_node) => SerializedAssetGraphNode::Dependency {
        value: SerializedDependency {
          id: dependency_node.dependency.id(),
          dependency: dependency_node.dependency.as_ref().clone(),
        },
        has_deferred: dependency_node.state == DependencyState::Deferred,
      },
    })
  }

  for (from, to) in asset_graph.edges() {
    let Some(new_addr_from) = nx_to_index.get(&(from as u32).into()) else {
      panic!();
    };

    let Some(new_addr_to) = nx_to_index.get(&(to as u32).into()) else {
      panic!();
    };

    edges.push((new_addr_from.clone(), new_addr_to.clone()));
  }

  edges.sort_by(|a, b| format!("{}{}", a.0, a.1).cmp(&format!("{}{}", b.0, b.1)));

  let edges = edges
    .into_iter()
    .flat_map(|v| vec![v.0, v.1])
    .collect::<Vec<u32>>();

  Ok(SerializedAssetGraphResult { edges, nodes })
}

pub fn serialize_asset_graph(
  asset_graph: &AssetGraph,
  max_str_len: usize,
) -> serde_json::Result<Vec<String>> {
  let mut nodes: Vec<String> = Vec::new();
  let mut curr_node = String::default();

  for node in asset_graph.nodes() {
    let serialized_node = match node {
      AssetGraphNode::Root => SerializedAssetGraphNode::Root,
      AssetGraphNode::Entry => SerializedAssetGraphNode::Entry,
      AssetGraphNode::Asset(asset_node) => SerializedAssetGraphNode::Asset {
        value: asset_node.asset.clone(),
      },
      AssetGraphNode::Dependency(dependency_node) => SerializedAssetGraphNode::Dependency {
        value: SerializedDependency {
          id: dependency_node.dependency.id(),
          dependency: dependency_node.dependency.as_ref().clone(),
        },
        has_deferred: dependency_node.state == DependencyState::Deferred,
      },
    };

    let str = serde_json::to_string(&serialized_node)?;
    if curr_node.len() + str.len() < (max_str_len - 3) {
      if !curr_node.is_empty() {
        curr_node.push(',');
      }
      curr_node.push_str(&str);
    } else {
      // Add the existing node now as it has reached the max JavaScript string size
      nodes.push(format!("[{curr_node}]"));
      curr_node = str;
    }
  }

  // Add the current node if it did not overflow in size
  if curr_node.len() < (max_str_len - 3) {
    nodes.push(format!("[{curr_node}]"));
  }

  Ok(nodes)
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use serde_json::{json, Value};

  use super::*;

  #[test]
  fn serialize_nodes_handles_max_size() -> anyhow::Result<()> {
    let mut graph = AssetGraph::new();

    let entry = graph.add_entry_dependency(Dependency {
      specifier: String::from("entry"),
      ..Dependency::default()
    });

    let entry_asset = graph.add_asset(Asset {
      file_path: PathBuf::from("entry"),
      ..Asset::default()
    });

    graph.add_edge(&entry, &entry_asset);

    for i in 1..100 {
      let node_index = graph.add_dependency(Dependency {
        specifier: format!("dependency-{}", i),
        ..Dependency::default()
      });
      graph.add_edge(&entry_asset, &node_index);
    }

    let max_str_len = 10000;
    let nodes = serialize_asset_graph(&graph, max_str_len)?;

    assert_eq!(nodes.len(), 7);

    // Assert each string is less than the max size
    for node in nodes.iter() {
      assert!(node.len() < max_str_len);
    }

    // Assert all the nodes are included and in the correct order
    let first_entry = serde_json::from_str::<Value>(&nodes[0])?;
    let first_entry = first_entry.as_array().unwrap();

    assert_eq!(get_type(&first_entry[0]), json!("root"));
    assert_eq!(get_dependency(&first_entry[1]), Some(json!("entry")));
    assert_eq!(get_asset(&first_entry[2]), Some(json!("entry")));

    for i in 1..first_entry.len() - 2 {
      assert_eq!(
        get_dependency(&first_entry[i + 2]),
        Some(json!(format!("dependency-{}", i)))
      );
    }

    let mut specifier = first_entry.len() - 2;
    for node in nodes[1..].iter() {
      let entry = serde_json::from_str::<Value>(&node)?;
      let entry = entry.as_array().unwrap();

      for value in entry {
        assert_eq!(
          get_dependency(&value),
          Some(json!(format!("dependency-{}", specifier)))
        );

        specifier += 1;
      }
    }

    Ok(())
  }

  fn get_type(node: &Value) -> Value {
    node.get("type").unwrap().to_owned()
  }

  fn get_dependency(value: &Value) -> Option<Value> {
    assert_eq!(get_type(&value), json!("dependency"));

    value
      .get("value")
      .unwrap()
      .get("dependency")
      .unwrap()
      .get("specifier")
      .map(|s| s.to_owned())
  }

  fn get_asset(value: &Value) -> Option<Value> {
    assert_eq!(get_type(&value), json!("asset"));

    value
      .get("value")
      .unwrap()
      .get("filePath")
      .map(|s| s.to_owned())
  }
}