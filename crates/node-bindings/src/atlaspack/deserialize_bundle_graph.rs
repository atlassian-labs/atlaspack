use std::collections::{HashMap, HashSet};

use atlaspack_core::{
  bundle_graph::BundleGraph,
  types::{Asset, Dependency},
};
use napi::{
  Env, JsObject,
  bindgen_prelude::{Array, Uint32Array},
};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

// #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct SerializedContentGraph<N, E> {
//   pub nodes: Vec<N>,
//   pub adjacency_list: SerializedAdjacencyList<E>,
// }
// pub struct SerializedAdjacencyList<E> {
//   pub edges: Vec<E>,
// }

// #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct SerializedJsBundleGraph {
//   pub graph: SerializedContentGraph<JsObject, u16>,
//   pub bundle_content_hashes: HashMap<String, String>,
//   pub asset_public_ids: HashSet<String>,
//   pub public_id_by_asset_id: HashMap<String, String>,
//   pub conditions: HashMap<String, Condition>,
// }

// #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Condition {
//   pub public_id: String;
//   pub assets: HashSet<Asset>;
//   pub key: String;
//   pub if_true_dependency: Dependency;
//   pub if_false_dependency: Dependency;
// }

#[napi]
pub fn deserialize_bundle_graph(env: Env, js_bundle_graph: JsObject) -> () {
  let bundle_graph: BundleGraph = BundleGraph::new();

  let graph: JsObject = js_bundle_graph.get_named_property("graph").unwrap();
  let adjacency_list: JsObject = graph.get_named_property("adjacencyList").unwrap();
  let edges: Uint32Array = adjacency_list.get_named_property("edges").unwrap();
  let nodes: Array = graph.get_named_property("nodes").unwrap();
  for i in 0..nodes.len() {
    let node: JsObject = nodes.get(i).unwrap().unwrap();
    let node_type: String = node.get_named_property("type").unwrap();
    dbg!(node_type);
  }
}
