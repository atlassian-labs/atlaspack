use atlaspack_core::types::{
  AssetNode, BundleGraph, BundleGraphNode, BundleGroupNode, BundleNode, DependencyNode,
  EntryFileNode, EntrySpecifierNode, RootNode,
};
use napi::{
  Env, JsObject, JsUnknown,
  bindgen_prelude::{Array, Uint32Array},
};
use napi_derive::napi;

/// Deserialize a single BundleGraphNode from a JsObject
#[tracing::instrument(level = "trace", skip_all)]
pub fn deserialize_bundle_graph_node(env: &Env, node: JsObject) -> napi::Result<BundleGraphNode> {
  // First, get the type field to determine which variant to deserialize
  let node_type: String = node.get_named_property("type").map_err(|e| {
    napi::Error::new(
      napi::Status::InvalidArg,
      format!("Failed to get 'type' field: {}", e),
    )
  })?;

  let node_unknown: JsUnknown = node.into_unknown();

  // Deserialize based on the type field
  match node_type.as_str() {
    "asset" => {
      let asset_node: AssetNode = env.from_js_value(node_unknown)?;
      Ok(BundleGraphNode::Asset(asset_node))
    }
    "dependency" => {
      let dep_node: DependencyNode = env.from_js_value(node_unknown).map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to deserialize dependency node: {}", e),
        )
      })?;
      Ok(BundleGraphNode::Dependency(dep_node))
    }
    "entry_specifier" => {
      let entry_spec_node: EntrySpecifierNode = env.from_js_value(node_unknown)?;
      Ok(BundleGraphNode::EntrySpecifier(entry_spec_node))
    }
    "entry_file" => {
      let entry_file_node: EntryFileNode = env.from_js_value(node_unknown)?;
      Ok(BundleGraphNode::EntryFile(entry_file_node))
    }
    "root" => {
      let root_node: RootNode = env.from_js_value(node_unknown)?;
      Ok(BundleGraphNode::Root(root_node))
    }
    "bundle_group" => {
      let bundle_group_node: BundleGroupNode = env.from_js_value(node_unknown)?;
      Ok(BundleGraphNode::BundleGroup(bundle_group_node))
    }
    "bundle" => {
      let bundle_node: BundleNode = env.from_js_value(node_unknown)?;
      Ok(BundleGraphNode::Bundle(bundle_node))
    }
    _ => Err(napi::Error::new(
      napi::Status::InvalidArg,
      format!("Unknown node type: {}", node_type),
    )),
  }
}

#[napi]
#[tracing::instrument(level = "info", skip_all)]
pub fn deserialize_bundle_graph(
  env: Env,
  js_bundle_graph: JsObject,
  raw_edges: Vec<(u32, u32)>,
) -> napi::Result<()> {
  let mut bundle_graph: BundleGraph = BundleGraph::new();

  let graph: JsObject = js_bundle_graph.get_named_property("graph").map_err(|e| {
    napi::Error::new(
      napi::Status::GenericFailure,
      format!("Failed to get graph: {}", e),
    )
  })?;
  let adjacency_list: JsObject = graph.get_named_property("adjacencyList").map_err(|e| {
    napi::Error::new(
      napi::Status::GenericFailure,
      format!("Failed to get adjacencyList: {}", e),
    )
  })?;
  let _edges: Uint32Array = adjacency_list.get_named_property("edges").map_err(|e| {
    napi::Error::new(
      napi::Status::GenericFailure,
      format!("Failed to get edges: {}", e),
    )
  })?;
  let nodes: Array = graph.get_named_property("nodes").map_err(|e| {
    napi::Error::new(
      napi::Status::GenericFailure,
      format!("Failed to get nodes: {}", e),
    )
  })?;

  for i in 0..nodes.len() {
    let node_unknown: JsUnknown = nodes
      .get::<JsUnknown>(i)
      .map_err(|e| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to get node at index {}: {}", i, e),
        )
      })?
      .ok_or_else(|| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Node at index {} is null or undefined", i),
        )
      })?;

    let node: JsObject = node_unknown.coerce_to_object().map_err(|e| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("Failed to convert node at index {} to JsObject: {:?}", i, e),
      )
    })?;

    match deserialize_bundle_graph_node(&env, node) {
      Ok(deserialized_node) => {
        bundle_graph.add_node(i, deserialized_node);
      }
      Err(e) => {
        return Err(napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to deserialize node at index {}: {}", i, e),
        ));
      }
    }
  }

  for (from, to) in raw_edges {
    tracing::trace!("Adding edge from {} to {}", from, to);
    bundle_graph.add_edge(from, to);
  }

  bundle_graph.traverse_bundles(|bundle| {
    dbg!("Bundle: {:?}", &bundle.name);
  });

  Ok(())
}
