use napi::{Env, JsObject};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use anyhow::anyhow;

use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode, DependencyState};

/// Bundle graph serialization format for JS.
///
/// This is analogous to `serialize_asset_graph`, but produces a *bundle graph* content graph
/// including asset/dependency nodes plus bundle/bundle_group nodes.
///
/// Returns a JS object:
/// ```ts
/// type SerializedBundleGraph = {
///   nodes: Array<JsBuffer>; // JSON-serialized nodes
///   edges: Array<number>;  // flat triples: [from, to, type, from, to, type, ...]
///   publicIdByAssetId: Record<string, string>;
///   assetPublicIds: Array<string>;
///   hadPreviousGraph: boolean;
/// }
/// ```
#[tracing::instrument(level = "info", skip_all)]
pub fn serialize_bundle_graph(
  env: &Env,
  asset_graph: &AssetGraph,
  bundle_nodes: &Vec<String>,
  bundle_edges: &Vec<u32>,
  public_id_by_asset_id: &std::collections::HashMap<String, String>,
  asset_public_ids: &Vec<String>,
  had_previous_graph: bool,
) -> anyhow::Result<JsObject> {
  let mut napi_bundle_graph = env.create_object()?;

  // Serialize asset/dependency/root nodes from the asset graph first.
  let asset_nodes: Vec<&AssetGraphNode> = asset_graph.nodes().collect();

  let mut node_json_blobs: Vec<serde_json::Result<Vec<u8>>> = asset_nodes
    .par_iter()
    .map(|item| {
      Ok(serde_json::to_vec(&match item {
        AssetGraphNode::Root => SerializedBundleGraphNode::Root,
        AssetGraphNode::Entry => SerializedBundleGraphNode::Entry,
        AssetGraphNode::Asset(asset) => SerializedBundleGraphNode::Asset { value: asset.as_ref() },
        AssetGraphNode::Dependency(dependency) => {
          let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dependency.id()) else {
            return Err(anyhow!("Dependency node not found for id: {}", dependency.id()));
          };
          let dep_state = asset_graph.get_dependency_state(dep_node_id);
          SerializedBundleGraphNode::Dependency {
            value: SerializedDependency {
              id: dependency.id(),
              dependency: dependency.as_ref(),
            },
            has_deferred: *dep_state == DependencyState::Deferred,
          }
        }
      }))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

  // Append bundle-specific nodes (bundle_group + bundle) that were produced by the bundler.
  for n in bundle_nodes {
    node_json_blobs.push(Ok(n.as_bytes().to_vec()));
  }

  let mut js_nodes = env.create_array_with_length(node_json_blobs.len())?;
  for (i, node_bytes) in node_json_blobs.into_iter().enumerate() {
    let js_buffer = env.create_buffer_with_data(node_bytes?)?;
    let js_buffer = js_buffer.into_unknown();
    js_nodes.set_element(i as u32, js_buffer)?;
  }

  napi_bundle_graph.set_named_property("nodes", js_nodes)?;

  // Emit edges as typed triples: [from, to, type, ...]
  // First include all asset graph edges as null edges.
  let asset_edges = asset_graph.edges();
  let mut edges_flat: Vec<u32> = Vec::with_capacity(asset_edges.len() / 2 * 3 + bundle_edges.len());
  for pair in asset_edges.chunks_exact(2) {
    edges_flat.push(pair[0]);
    edges_flat.push(pair[1]);
    edges_flat.push(1);
  }
  edges_flat.extend_from_slice(bundle_edges);

  napi_bundle_graph.set_named_property("edges", edges_flat)?;
  napi_bundle_graph.set_named_property("publicIdByAssetId", env.to_js_value(public_id_by_asset_id)?)?;
  napi_bundle_graph.set_named_property("assetPublicIds", env.to_js_value(asset_public_ids)?)?;
  napi_bundle_graph.set_named_property("hadPreviousGraph", had_previous_graph)?;

  Ok(napi_bundle_graph)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDependency<'a> {
  id: String,
  dependency: &'a atlaspack_core::types::Dependency,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedBundleGraphNode<'a> {
  Root,
  Entry,
  Asset { value: &'a atlaspack_core::types::Asset },
  Dependency { value: SerializedDependency<'a>, has_deferred: bool },
}
