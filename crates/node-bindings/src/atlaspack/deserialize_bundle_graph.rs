use atlaspack_core::types::{
  AssetNode, BundleGraphNode, BundleGroupNode, BundleNode, DependencyNode, EntryFileNode,
  EntrySpecifierNode, RootNode,
};
use napi::{Env, JsObject};

/// Helper function to extract field information from error messages
fn extract_field_info(error_msg: &str) -> Option<String> {
  // Try to find common serde error patterns that mention fields
  // Pattern: "missing field `field_name`" or "unknown field `field_name`"
  // Also look for patterns like "invalid type: expected X, found Y at line X column Y"
  // which might be followed by field context

  // Pattern 1: "missing field `field_name`" or "unknown field `field_name`"
  if (error_msg.contains("missing field") || error_msg.contains("unknown field"))
    && let Some(start) = error_msg.find("`")
    && let Some(end) = error_msg[start + 1..].find("`")
  {
    return Some(error_msg[start + 1..start + 1 + end].to_string());
  }

  // Pattern 2: Look for field names in quotes or after common keywords
  // e.g., "field 'fieldName': invalid type" or "at field 'fieldName'"
  if let Some(field_start) = error_msg.find("field '") {
    let after_field = &error_msg[field_start + 7..];
    if let Some(field_end) = after_field.find("'") {
      return Some(after_field[..field_end].to_string());
    }
  }

  // Pattern 3: Look for field names in backticks anywhere in the message
  // This is a fallback to catch any field name mentioned in backticks
  if let Some(start) = error_msg.find("`")
    && let Some(end) = error_msg[start + 1..].find("`")
  {
    let potential_field = &error_msg[start + 1..start + 1 + end];
    // Only return if it looks like a field name (alphanumeric with underscores/camelCase)
    if potential_field
      .chars()
      .all(|c| c.is_alphanumeric() || c == '_')
    {
      return Some(potential_field.to_string());
    }
  }

  None
}

/// Helper function to create a detailed deserialization error message
fn create_deserialization_error(
  node_type: &str,
  node_id: Option<&str>,
  error: napi::Error,
) -> napi::Error {
  let error_msg = error.to_string();
  let field_info = extract_field_info(&error_msg);

  let detailed_msg = match (node_id, field_info) {
    (Some(id), Some(field)) => {
      format!(
        "Failed to deserialize {} node (id: '{}', field: '{}'): {}",
        node_type, id, field, error_msg
      )
    }
    (Some(id), None) => {
      format!(
        "Failed to deserialize {} node (id: '{}'): {}",
        node_type, id, error_msg
      )
    }
    (None, Some(field)) => {
      format!(
        "Failed to deserialize {} node (field: '{}'): {}",
        node_type, field, error_msg
      )
    }
    (None, None) => {
      format!("Failed to deserialize {} node: {}", node_type, error_msg)
    }
  };

  napi::Error::new(napi::Status::GenericFailure, detailed_msg)
}

/// Deserialize a single BundleGraphNode from a JsObject
pub fn deserialize_bundle_graph_node(env: &Env, node: &JsObject) -> napi::Result<BundleGraphNode> {
  // First, get the type field to determine which variant to deserialize
  let node_type: String = node.get_named_property("type").map_err(|e| {
    napi::Error::new(
      napi::Status::InvalidArg,
      format!("Failed to get 'type' field: {}", e),
    )
  })?;
  // Deserialize based on the type field
  match node_type.as_str() {
    "asset" => {
      let asset_node: AssetNode = env
        .from_js_value::<AssetNode, _>(node)
        .map_err(|e| create_deserialization_error("asset", None, e))?;
      Ok(BundleGraphNode::Asset(asset_node))
    }
    "dependency" => {
      let dep_node: DependencyNode = env
        .from_js_value::<DependencyNode, _>(node)
        .map_err(|e| create_deserialization_error("dependency", None, e))?;
      Ok(BundleGraphNode::Dependency(dep_node))
    }
    "entry_specifier" => {
      let entry_spec_node: EntrySpecifierNode = env
        .from_js_value::<EntrySpecifierNode, _>(node)
        .map_err(|e| create_deserialization_error("entry_specifier", None, e))?;
      Ok(BundleGraphNode::EntrySpecifier(entry_spec_node))
    }
    "entry_file" => {
      let entry_file_node: EntryFileNode = env
        .from_js_value::<EntryFileNode, _>(node)
        .map_err(|e| create_deserialization_error("entry_file", None, e))?;
      Ok(BundleGraphNode::EntryFile(entry_file_node))
    }
    "root" => {
      let root_node: RootNode = env
        .from_js_value::<RootNode, _>(node)
        .map_err(|e| create_deserialization_error("root", None, e))?;
      Ok(BundleGraphNode::Root(root_node))
    }
    "bundle_group" => {
      let bundle_group_node: BundleGroupNode = env
        .from_js_value::<BundleGroupNode, _>(node)
        .map_err(|e| create_deserialization_error("bundle_group", None, e))?;
      Ok(BundleGraphNode::BundleGroup(bundle_group_node))
    }
    "bundle" => {
      let bundle_node: BundleNode = env
        .from_js_value::<BundleNode, _>(node)
        .map_err(|e| create_deserialization_error("bundle", None, e))?;
      Ok(BundleGraphNode::Bundle(bundle_node))
    }
    _ => Err(napi::Error::new(
      napi::Status::InvalidArg,
      format!("Unknown node type: {}", node_type),
    )),
  }
}

#[tracing::instrument(level = "info", skip_all)]
pub fn deserialize_bundle_graph(
  env: Env,
  nodes: Vec<JsObject>,
) -> anyhow::Result<Vec<BundleGraphNode>> {
  let node_count = nodes.len();

  let mut deserialized_nodes = Vec::with_capacity(node_count);
  for i in 0..node_count {
    let node: &JsObject = nodes.get(i).ok_or_else(|| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("Node at index {} is null or undefined", i),
      )
    })?;

    let deserialized_node = deserialize_bundle_graph_node(&env, node).map_err(|e| {
      // The error from deserialize_bundle_graph_node already includes node type and field info
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("Failed to deserialize node at index {}: {}", i, e),
      )
    })?;
    deserialized_nodes.push(deserialized_node);
  }
  Ok(deserialized_nodes)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_field_info_missing_field_backticks() {
    let error_msg = "missing field `specifier`";
    let result = extract_field_info(error_msg);
    assert_eq!(result, Some("specifier".to_string()));
  }

  #[test]
  fn test_extract_field_info_unknown_field_backticks() {
    let error_msg = "unknown field `unknownField`";
    let result = extract_field_info(error_msg);
    assert_eq!(result, Some("unknownField".to_string()));
  }

  #[test]
  fn test_extract_field_info_single_quotes() {
    let error_msg = "field 'envName': invalid type";
    let result = extract_field_info(error_msg);
    assert_eq!(result, Some("envName".to_string()));
  }

  #[test]
  fn test_extract_field_info_fallback_backticks() {
    // Fallback to any alphanumeric content in backticks
    let error_msg = "error deserializing `bundleBehavior` value";
    let result = extract_field_info(error_msg);
    assert_eq!(result, Some("bundleBehavior".to_string()));
  }

  #[test]
  fn test_extract_field_info_with_underscores() {
    let error_msg = "missing field `source_asset_id`";
    let result = extract_field_info(error_msg);
    assert_eq!(result, Some("source_asset_id".to_string()));
  }

  #[test]
  fn test_extract_field_info_no_field() {
    let error_msg = "invalid type: expected string, found number";
    let result = extract_field_info(error_msg);
    assert_eq!(result, None);
  }

  #[test]
  fn test_extract_field_info_empty_string() {
    let result = extract_field_info("");
    assert_eq!(result, None);
  }

  #[test]
  fn test_extract_field_info_non_alphanumeric_in_backticks() {
    // Should not match if content in backticks is not alphanumeric
    let error_msg = "error at `line:col` position";
    let result = extract_field_info(error_msg);
    // "line:col" contains ':', not alphanumeric, so no match from fallback
    assert_eq!(result, None);
  }

  #[test]
  fn test_create_deserialization_error_with_id_and_field() {
    let original_error =
      napi::Error::new(napi::Status::GenericFailure, "missing field `specifier`");
    let result = create_deserialization_error("asset", Some("asset123"), original_error);

    let error_msg = result.to_string();
    assert!(error_msg.contains("asset"));
    assert!(error_msg.contains("asset123"));
    assert!(error_msg.contains("specifier"));
  }

  #[test]
  fn test_create_deserialization_error_with_id_only() {
    let original_error = napi::Error::new(
      napi::Status::GenericFailure,
      "invalid type: expected string",
    );
    let result = create_deserialization_error("dependency", Some("dep456"), original_error);

    let error_msg = result.to_string();
    assert!(error_msg.contains("dependency"));
    assert!(error_msg.contains("dep456"));
    assert!(error_msg.contains("invalid type"));
  }

  #[test]
  fn test_create_deserialization_error_with_field_only() {
    let original_error =
      napi::Error::new(napi::Status::GenericFailure, "unknown field `extraField`");
    let result = create_deserialization_error("bundle", None, original_error);

    let error_msg = result.to_string();
    assert!(error_msg.contains("bundle"));
    assert!(error_msg.contains("extraField"));
  }

  #[test]
  fn test_create_deserialization_error_minimal() {
    let original_error = napi::Error::new(napi::Status::GenericFailure, "parse error");
    let result = create_deserialization_error("root", None, original_error);

    let error_msg = result.to_string();
    assert!(error_msg.contains("root"));
    assert!(error_msg.contains("parse error"));
  }
}
