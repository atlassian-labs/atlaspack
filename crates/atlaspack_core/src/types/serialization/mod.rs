use std::collections::BTreeSet;

use crate::types::asset::Condition;
use crate::types::json::JSONObject;

/// Helper macro to deserialize a field with context on error.
///
/// Wraps `map.next_value()` with tracing and error context that includes
/// the field name being deserialized, making debugging much easier.
///
/// # Usage
/// ```ignore
/// use crate::types::serialization::deserialize_field;
///
/// // In a Visitor's visit_map implementation:
/// "fieldName" => some_var = Some(deserialize_field!(map, "fieldName", "TypeName")?),
/// ```
#[macro_export]
macro_rules! deserialize_field {
  ($map:expr, $field:expr, $type_name:expr) => {{
    $map.next_value().map_err(|e| {
      tracing::error!(field = $field, error = %e, concat!($type_name, " deserialization failed"));
      serde::de::Error::custom(format!("field '{}': {}", $field, e))
    })
  }};
}

pub use deserialize_field;

pub fn extract_val_default<T>(map: &mut serde_json::Map<String, serde_json::Value>, key: &str) -> T
where
  T: serde::de::DeserializeOwned + Default,
{
  extract_val(map, key).unwrap_or_default()
}

pub fn extract_val<T>(map: &mut serde_json::Map<String, serde_json::Value>, key: &str) -> Option<T>
where
  T: serde::de::DeserializeOwned,
{
  let val = map.get(key).and_then(|v| T::deserialize(v).ok());
  map.remove(key);
  val
}

/// Fields that are stored in the Asset's `meta` JSON object during serialization
/// but are extracted into dedicated struct fields during deserialization.
///
/// This struct ensures consistency between:
/// - The custom Asset `Deserialize` implementation
/// - The RPC transformer when constructing Assets from Node.js results
///
/// If you add a new field that should be stored in meta, update:
/// 1. This struct and `extract_asset_meta_fields()`
/// 2. The `Asset::serialize()` implementation to insert the field into meta
#[derive(Debug, Default)]
pub struct ExtractedAssetMetaFields {
  pub conditions: BTreeSet<Condition>,
  pub has_cjs_exports: bool,
  pub has_node_replacements: bool,
  pub is_constant_module: bool,
  pub should_wrap: bool,
  pub static_exports: bool,
  pub css_dependency_type: Option<String>,
  pub empty_file_star_reexport: Option<bool>,
  pub has_dependencies: Option<bool>,
  pub has_references: Option<bool>,
  pub inline_type: Option<String>,
  pub interpreter: Option<String>,
  pub packaging_id: Option<String>,
}

/// Extracts Asset-specific fields from a meta JSON object.
///
/// These fields are stored in the `meta` object during serialization for JavaScript
/// interoperability, but are represented as dedicated struct fields in Rust.
///
/// This function modifies the input `meta` map by removing the extracted keys,
/// leaving only custom/plugin-specific metadata.
pub fn extract_asset_meta_fields(meta: &mut JSONObject) -> ExtractedAssetMetaFields {
  ExtractedAssetMetaFields {
    conditions: extract_val_default(meta, "conditions"),
    has_cjs_exports: extract_val_default(meta, "hasCJSExports"),
    has_node_replacements: extract_val_default(meta, "has_node_replacements"),
    is_constant_module: extract_val_default(meta, "isConstantModule"),
    should_wrap: extract_val_default(meta, "shouldWrap"),
    static_exports: extract_val_default(meta, "staticExports"),
    css_dependency_type: extract_val(meta, "type"),
    empty_file_star_reexport: extract_val(meta, "emptyFileStarReexport"),
    has_dependencies: extract_val(meta, "hasDependencies"),
    has_references: extract_val(meta, "hasReferences"),
    inline_type: extract_val(meta, "inlineType"),
    interpreter: extract_val(meta, "interpreter"),
    packaging_id: extract_val(meta, "id"),
  }
}
