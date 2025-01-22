use napi::{Env, JsObject};
use serde::Serialize;

use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode, DependencyState};
use atlaspack_core::types::{Asset, Dependency};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDependency {
  id: String,
  dependency: Dependency,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedAssetGraphNode {
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

/// Returns
/// ```typescript
/// type SerializedAssetGraph = {
///   edges: Array<number>,
///   nodes: Array<JsBuffer>,
/// }
/// ```
pub fn serialize_asset_graph(env: &Env, asset_graph: &AssetGraph) -> anyhow::Result<JsObject> {
  let mut js_nodes = env.create_empty_array()?;

  for (i, node) in asset_graph.nodes().enumerate() {
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

    let node_bytes = serde_json::to_vec(&serialized_node)?;
    let js_buffer = env.create_buffer_with_data(node_bytes)?;
    let js_buffer = js_buffer.into_unknown();

    js_nodes.set_element(i as u32, js_buffer)?;
  }

  let mut napi_asset_graph = env.create_object()?;
  napi_asset_graph.set_named_property("edges", asset_graph.edges())?;
  napi_asset_graph.set_named_property("nodes", js_nodes)?;

  Ok(napi_asset_graph)
}
