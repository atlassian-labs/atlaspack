use napi::{Env, JsObject};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use anyhow::anyhow;

use atlaspack_core::asset_graph::{
  AssetGraph, AssetGraphNode, DependencyState, FinalizedSymbolTracker, UsedSymbol,
};
use atlaspack_core::types::{Asset, Dependency};

/// Returns
/// ```typescript
/// type SerializedAssetGraph = {
///   edges: Array<number>,
///   nodes: Array<JsBuffer>,
/// }
/// ```
#[tracing::instrument(level = "info", skip_all)]
pub fn serialize_asset_graph(
  env: &Env,
  symbol_tracker: Option<&FinalizedSymbolTracker>,
  asset_graph: &AssetGraph,
  had_previous_graph: bool,
) -> anyhow::Result<JsObject> {
  let mut napi_asset_graph = env.create_object()?;

  napi_asset_graph.set_named_property(
    "nodes",
    serialize_asset_graph_nodes(
      env,
      symbol_tracker,
      asset_graph,
      &asset_graph.new_nodes().collect(),
    )?,
  )?;

  napi_asset_graph.set_named_property(
    "updates",
    serialize_asset_graph_nodes(
      env,
      symbol_tracker,
      asset_graph,
      &asset_graph.updated_nodes().collect(),
    )?,
  )?;

  if !asset_graph.safe_to_skip_bundling {
    napi_asset_graph.set_named_property("edges", asset_graph.edges())?;
  }

  napi_asset_graph.set_named_property("safeToSkipBundling", asset_graph.safe_to_skip_bundling)?;
  napi_asset_graph.set_named_property("hadPreviousGraph", had_previous_graph)?;

  Ok(napi_asset_graph)
}

fn serialize_asset_graph_nodes(
  env: &Env,
  symbol_tracker: Option<&FinalizedSymbolTracker>,
  asset_graph: &AssetGraph,
  nodes: &Vec<&AssetGraphNode>,
) -> anyhow::Result<JsObject> {
  // Serialize graph nodes in parallel
  let serialized_nodes = nodes
    .par_iter()
    .map(|item| {
      Ok(serde_json::to_vec(&match item {
        AssetGraphNode::Root => SerializedAssetGraphNode::Root,
        AssetGraphNode::Entry => SerializedAssetGraphNode::Entry,
        AssetGraphNode::Asset(asset) => SerializedAssetGraphNode::Asset {
          value: asset.as_ref(),
        },
        AssetGraphNode::Dependency(dependency) => {
          let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dependency.id()) else {
            return Err(anyhow!(
              "Dependency node not found for id: {}",
              dependency.id()
            ));
          };

          let dep_state = asset_graph.get_dependency_state(dep_node_id);

          let used_symbols_up: Option<Vec<&UsedSymbol>> = symbol_tracker
            .and_then(|tracker| tracker.get_used_symbols_for_dependency(&dependency.id))
            .map(|u| u.values().collect());

          SerializedAssetGraphNode::Dependency {
            value: SerializedDependency {
              id: dependency.id(),
              dependency: dependency.as_ref(),
            },
            has_deferred: *dep_state == DependencyState::Deferred,
            used_symbols_up,
          }
        }
      }))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

  // Collect the results into a JavaScript Array
  // TODO NAPI3 allows removing this
  let mut js_nodes = env.create_array_with_length(serialized_nodes.len())?;

  for (i, node_bytes) in serialized_nodes.into_iter().enumerate() {
    let js_buffer = env.create_buffer_with_data(node_bytes?)?;
    let js_buffer = js_buffer.into_unknown();

    js_nodes.set_element(i as u32, js_buffer)?;
  }

  Ok(js_nodes)
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
    used_symbols_up: Option<Vec<&'a UsedSymbol>>,
  },
}
