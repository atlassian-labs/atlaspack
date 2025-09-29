use napi::{Env, JsObject};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode, DependencyState};
use atlaspack_core::types::{Asset, Dependency};

/// Returns
/// ```typescript
/// type SerializedAssetGraph = {
///   edges: Array<number>,
///   nodes: Array<JsBuffer>,
/// }
/// ```
#[tracing::instrument(level = "info", skip_all)]
pub fn serialize_asset_graph(env: &Env, asset_graph: &AssetGraph) -> anyhow::Result<JsObject> {
  // Serialize graph nodes in parallel
  let nodes = asset_graph
    .nodes()
    .collect::<Vec<_>>()
    .par_iter()
    .map(|item| {
      serde_json::to_vec(&match item {
        AssetGraphNode::Root => SerializedAssetGraphNode::Root,
        AssetGraphNode::Entry => SerializedAssetGraphNode::Entry,
        AssetGraphNode::Asset(asset) => SerializedAssetGraphNode::Asset {
          value: asset.as_ref(),
        },
        AssetGraphNode::Dependency(dependency) => SerializedAssetGraphNode::Dependency {
          value: SerializedDependency {
            id: dependency.id(),
            dependency: dependency.as_ref(),
          },
          // TODO: Add some tracking of id to node id in the asset_graph
          has_deferred: *asset_graph.get_dependency_state(&0_usize) == DependencyState::Deferred,
        },
      })
    })
    .collect::<Vec<_>>();

  // Collect the results into a JavaScript Array
  // TODO NAPI3 allows removing this
  let mut js_nodes = env.create_array_with_length(nodes.len())?;

  for (i, node_bytes) in nodes.into_iter().enumerate() {
    let js_buffer = env.create_buffer_with_data(node_bytes?)?;
    let js_buffer = js_buffer.into_unknown();

    js_nodes.set_element(i as u32, js_buffer)?;
  }

  // TODO use a napi_derive::napi(object) with NAPI3
  let mut napi_asset_graph = env.create_object()?;
  napi_asset_graph.set_named_property("edges", asset_graph.edges())?;
  napi_asset_graph.set_named_property("nodes", js_nodes)?;

  Ok(napi_asset_graph)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDependency<'a> {
  id: String,
  dependency: &'a Dependency,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedAssetGraphNode<'a> {
  Root,
  Entry,
  Asset {
    value: &'a Asset,
  },
  Dependency {
    value: SerializedDependency<'a>,
    has_deferred: bool,
  },
}
