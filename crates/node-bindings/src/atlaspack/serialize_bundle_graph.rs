use napi::{Env, JsObject};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use anyhow::anyhow;

use atlaspack_core::bundle_graph::native_bundle_graph::{NativeBundleGraph, NativeBundleGraphNode};
use atlaspack_core::types::{Asset, Bundle, Dependency, Target};

/// Returns
/// ```typescript
/// type SerializedBundleGraph = {
///   edges: Array<number>,
///   nodes: Array<JsBuffer>,
///   publicIdByAssetId: Record<string, string>,
///   assetPublicIds: Array<string>,
///   hadPreviousGraph: boolean,
/// }
/// ```
#[tracing::instrument(level = "info", skip_all)]
pub fn serialize_bundle_graph(
  env: &Env,
  bundle_graph: &NativeBundleGraph,
  had_previous_graph: bool,
) -> anyhow::Result<JsObject> {
  let mut napi_bundle_graph = env.create_object()?;

  // Serialize nodes in parallel
  let nodes: Vec<&NativeBundleGraphNode> = bundle_graph.nodes().collect();

  let serialized_nodes = nodes
    .par_iter()
    .map(|item| {
      Ok(serde_json::to_vec(&match item {
        NativeBundleGraphNode::Root => SerializedBundleGraphNode::Root,
        NativeBundleGraphNode::Asset(asset) => SerializedBundleGraphNode::Asset {
          value: asset.as_ref(),
        },
        NativeBundleGraphNode::Dependency(dep) => SerializedBundleGraphNode::Dependency {
          value: SerializedDependency {
            id: dep.id(),
            dependency: dep.as_ref(),
          },
          has_deferred: false,
        },
        NativeBundleGraphNode::BundleGroup {
          target,
          entry_asset_id,
        } => SerializedBundleGraphNode::BundleGroup {
          id: format!("bundle_group:{}{}", target.name, entry_asset_id),
          target,
          entry_asset_id,
        },
        NativeBundleGraphNode::Bundle(bundle) => SerializedBundleGraphNode::Bundle {
          id: bundle.id.clone(),
          bundle,
        },
      }))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

  let mut js_nodes = env.create_array_with_length(serialized_nodes.len())?;
  for (i, node_bytes) in serialized_nodes.into_iter().enumerate() {
    let js_buffer = env.create_buffer_with_data(node_bytes?)?;
    let js_buffer = js_buffer.into_unknown();
    js_nodes.set_element(i as u32, js_buffer)?;
  }

  napi_bundle_graph.set_named_property("nodes", js_nodes)?;

  // Edges: flat triples [from,to,type,...]
  let mut edges_flat: Vec<u32> = Vec::new();
  for e in bundle_graph.graph.edge_references() {
    let from = *bundle_graph
      .graph
      .node_weight(e.source())
      .ok_or_else(|| anyhow!("Missing from node"))?;
    let to = *bundle_graph
      .graph
      .node_weight(e.target())
      .ok_or_else(|| anyhow!("Missing to node"))?;
    edges_flat.push(from as u32);
    edges_flat.push(to as u32);
    edges_flat.push(*e.weight() as u8 as u32);
  }
  napi_bundle_graph.set_named_property("edges", edges_flat)?;

  napi_bundle_graph.set_named_property(
    "publicIdByAssetId",
    env.to_js_value(&bundle_graph.public_id_by_asset_id)?,
  )?;
  napi_bundle_graph.set_named_property(
    "assetPublicIds",
    env.to_js_value(
      &bundle_graph
        .asset_public_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>(),
    )?,
  )?;
  napi_bundle_graph.set_named_property("hadPreviousGraph", had_previous_graph)?;

  Ok(napi_bundle_graph)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDependency<'a> {
  id: String,
  dependency: &'a Dependency,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedBundleGraphNode<'a> {
  Root,
  Asset {
    value: &'a Asset,
  },
  Dependency {
    value: SerializedDependency<'a>,
    has_deferred: bool,
  },
  BundleGroup {
    id: String,
    #[serde(rename = "target")]
    target: &'a Target,
    entry_asset_id: &'a String,
  },
  Bundle {
    id: String,
    #[serde(rename = "value")]
    bundle: &'a Bundle,
  },
}
