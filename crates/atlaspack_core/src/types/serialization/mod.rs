use std::collections::BTreeSet;

use crate::types::Symbol;
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

/// Deserialize symbols from either an array or a map format.
///
/// JavaScript represents symbols as `Map<exported, {local, loc, isWeak, meta}>`,
/// which serializes to a JSON object with exported names as keys.
/// Rust expects `Vec<Symbol>` with the exported name as a field.
///
/// This function handles both formats:
/// - Array: `[{exported, local, loc, ...}, ...]` - used by Rust serialization
/// - Object/Map: `{exported_name: {local, loc, ...}, ...}` - used by JS serialization
pub fn deserialize_symbols_field(value: serde_json::Value) -> Result<Option<Vec<Symbol>>, String> {
  if value.is_null() {
    return Ok(None);
  }

  if value.is_array() {
    // Standard sequence deserialization
    serde_json::from_value::<Vec<Symbol>>(value)
      .map(Some)
      .map_err(|e| e.to_string())
  } else if value.is_object() {
    // Map representation: key is "exported", value contains other fields
    let mut symbols_vec = Vec::new();
    let obj = value
      .as_object()
      .ok_or_else(|| "Expected object for symbols map".to_string())?;

    for (exported_name, val) in obj {
      // The value is {local, loc, isWeak, meta: {isEsm, isStaticBindingSafe}}
      // We need to extract these and build a Symbol
      let local = val
        .get("local")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

      let loc = val.get("loc").and_then(|v| {
        if v.is_null() {
          None
        } else {
          serde_json::from_value(v.clone()).ok()
        }
      });

      let is_weak = val.get("isWeak").and_then(|v| v.as_bool()).unwrap_or(false);

      // Meta contains isEsm (isEsmExport) and isStaticBindingSafe
      let meta = val.get("meta");
      let is_esm_export = meta
        .and_then(|m| m.get("isEsm"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
      let is_static_binding_safe = meta
        .and_then(|m| m.get("isStaticBindingSafe"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

      symbols_vec.push(Symbol {
        exported: exported_name.clone(),
        local,
        loc,
        is_weak,
        is_esm_export,
        self_referenced: false,
        is_static_binding_safe,
      });
    }
    Ok(Some(symbols_vec))
  } else {
    Err("symbols must be an array, object, or null".to_string())
  }
}

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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::asset::Condition;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_extract_val_string() {
    let mut meta = serde_json::Map::new();
    meta.insert("name".to_string(), serde_json::json!("test-value"));
    meta.insert("other".to_string(), serde_json::json!("keep-me"));

    let result: Option<String> = extract_val(&mut meta, "name");

    assert_eq!(result, Some("test-value".to_string()));
    // Key should be removed from the map
    assert!(!meta.contains_key("name"));
    // Other keys remain
    assert!(meta.contains_key("other"));
  }

  #[test]
  fn test_extract_val_missing_key() {
    let mut meta = serde_json::Map::new();
    meta.insert("existing".to_string(), serde_json::json!("value"));

    let result: Option<String> = extract_val(&mut meta, "missing");

    assert_eq!(result, None);
    // Original key still exists
    assert!(meta.contains_key("existing"));
  }

  #[test]
  fn test_extract_val_wrong_type() {
    let mut meta = serde_json::Map::new();
    meta.insert("number".to_string(), serde_json::json!(42));

    // Try to extract as String (should fail type conversion)
    let result: Option<String> = extract_val(&mut meta, "number");

    // Fails to deserialize, but key is still removed
    assert_eq!(result, None);
  }

  #[test]
  fn test_extract_val_number() {
    let mut meta = serde_json::Map::new();
    meta.insert("count".to_string(), serde_json::json!(42));

    let result: Option<i32> = extract_val(&mut meta, "count");

    assert_eq!(result, Some(42));
    assert!(!meta.contains_key("count"));
  }

  #[test]
  fn test_extract_val_boolean() {
    let mut meta = serde_json::Map::new();
    meta.insert("enabled".to_string(), serde_json::json!(true));

    let result: Option<bool> = extract_val(&mut meta, "enabled");

    assert_eq!(result, Some(true));
    assert!(!meta.contains_key("enabled"));
  }

  #[test]
  fn test_extract_val_default_present() {
    let mut meta = serde_json::Map::new();
    meta.insert("value".to_string(), serde_json::json!(123));

    let result: i32 = extract_val_default(&mut meta, "value");

    assert_eq!(result, 123);
    assert!(!meta.contains_key("value"));
  }

  #[test]
  fn test_extract_val_default_missing() {
    let mut meta = serde_json::Map::new();

    let result: i32 = extract_val_default(&mut meta, "missing");

    assert_eq!(result, 0); // Default for i32
  }

  #[test]
  fn test_extract_val_default_string() {
    let mut meta = serde_json::Map::new();

    let result: String = extract_val_default(&mut meta, "missing");

    assert_eq!(result, String::new()); // Default for String
  }

  #[test]
  fn test_extract_val_default_bool() {
    let mut meta = serde_json::Map::new();

    let result: bool = extract_val_default(&mut meta, "missing");

    assert!(!result); // Default for bool is false
  }

  #[test]
  fn test_extract_val_default_vec() {
    let mut meta = serde_json::Map::new();

    let result: Vec<String> = extract_val_default(&mut meta, "missing");

    assert_eq!(result, Vec::<String>::new());
  }

  #[test]
  fn test_extract_asset_meta_fields_all_present() {
    let mut meta = serde_json::Map::new();
    meta.insert(
      "conditions".to_string(),
      serde_json::json!([{ "key": "test", "ifTruePlaceholder": "a.js", "ifFalsePlaceholder": "b.js" }]),
    );
    meta.insert("hasCJSExports".to_string(), serde_json::json!(true));
    meta.insert("has_node_replacements".to_string(), serde_json::json!(true));
    meta.insert("isConstantModule".to_string(), serde_json::json!(true));
    meta.insert("shouldWrap".to_string(), serde_json::json!(true));
    meta.insert("staticExports".to_string(), serde_json::json!(true));
    meta.insert("type".to_string(), serde_json::json!("tag"));
    meta.insert(
      "emptyFileStarReexport".to_string(),
      serde_json::json!(false),
    );
    meta.insert("hasDependencies".to_string(), serde_json::json!(true));
    meta.insert("hasReferences".to_string(), serde_json::json!(true));
    meta.insert("inlineType".to_string(), serde_json::json!("string"));
    meta.insert(
      "interpreter".to_string(),
      serde_json::json!("#!/usr/bin/node"),
    );
    meta.insert("id".to_string(), serde_json::json!("pkg123"));
    // Add a custom field that should remain
    meta.insert(
      "customPlugin".to_string(),
      serde_json::json!("custom-value"),
    );

    let extracted = extract_asset_meta_fields(&mut meta);

    // Verify extracted values
    let expected_condition = Condition {
      key: "test".to_string(),
      if_true_placeholder: Some("a.js".to_string()),
      if_false_placeholder: Some("b.js".to_string()),
    };
    assert_eq!(extracted.conditions.len(), 1);
    assert!(extracted.conditions.contains(&expected_condition));
    assert!(extracted.has_cjs_exports);
    assert!(extracted.has_node_replacements);
    assert!(extracted.is_constant_module);
    assert!(extracted.should_wrap);
    assert!(extracted.static_exports);
    assert_eq!(extracted.css_dependency_type, Some("tag".to_string()));
    assert_eq!(extracted.empty_file_star_reexport, Some(false));
    assert_eq!(extracted.has_dependencies, Some(true));
    assert_eq!(extracted.has_references, Some(true));
    assert_eq!(extracted.inline_type, Some("string".to_string()));
    assert_eq!(extracted.interpreter, Some("#!/usr/bin/node".to_string()));
    assert_eq!(extracted.packaging_id, Some("pkg123".to_string()));

    // Custom field should still be present, all extracted keys removed
    assert!(meta.contains_key("customPlugin"));
    assert!(!meta.contains_key("conditions"));
    assert!(!meta.contains_key("hasCJSExports"));
    assert!(!meta.contains_key("type"));
    assert!(!meta.contains_key("id"));
  }

  #[test]
  fn test_extract_asset_meta_fields_empty() {
    let mut meta = serde_json::Map::new();

    let extracted = extract_asset_meta_fields(&mut meta);

    // All fields should have default values
    assert!(extracted.conditions.is_empty());
    assert!(!extracted.has_cjs_exports);
    assert!(!extracted.has_node_replacements);
    assert!(!extracted.is_constant_module);
    assert!(!extracted.should_wrap);
    assert!(!extracted.static_exports);
    assert_eq!(extracted.css_dependency_type, None);
    assert_eq!(extracted.empty_file_star_reexport, None);
    assert_eq!(extracted.has_dependencies, None);
    assert_eq!(extracted.has_references, None);
    assert_eq!(extracted.inline_type, None);
    assert_eq!(extracted.interpreter, None);
    assert_eq!(extracted.packaging_id, None);

    // Meta should be empty
    assert!(meta.is_empty());
  }

  #[test]
  fn test_extract_asset_meta_fields_partial() {
    let mut meta = serde_json::Map::new();
    meta.insert("hasCJSExports".to_string(), serde_json::json!(true));
    meta.insert("shouldWrap".to_string(), serde_json::json!(false));
    meta.insert(
      "pluginData".to_string(),
      serde_json::json!({ "key": "val" }),
    );

    let extracted = extract_asset_meta_fields(&mut meta);

    assert!(extracted.has_cjs_exports);
    assert!(!extracted.should_wrap);
    // Other fields should be defaults
    assert!(extracted.conditions.is_empty());
    assert_eq!(extracted.css_dependency_type, None);

    // Only pluginData should remain
    assert_eq!(meta.len(), 1);
    assert!(meta.contains_key("pluginData"));
  }

  #[test]
  fn test_deserialize_symbols_field_null() {
    let value = serde_json::json!(null);
    let result = deserialize_symbols_field(value);
    assert_eq!(result, Ok(None));
  }

  #[test]
  fn test_deserialize_symbols_field_array_format() {
    // Rust serialization format: array of Symbol objects
    let value = serde_json::json!([
      {
        "exported": "foo",
        "local": "$abc$foo",
        "loc": null,
        "isWeak": false,
        "isEsmExport": true,
        "selfReferenced": false,
        "isStaticBindingSafe": true
      },
      {
        "exported": "bar",
        "local": "$abc$bar",
        "loc": null,
        "isWeak": true,
        "isEsmExport": false,
        "selfReferenced": false,
        "isStaticBindingSafe": false
      }
    ]);

    let result = deserialize_symbols_field(value).unwrap().unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].exported, "foo");
    assert_eq!(result[0].local, "$abc$foo");
    assert!(!result[0].is_weak);
    assert!(result[0].is_esm_export);
    assert!(result[0].is_static_binding_safe);

    assert_eq!(result[1].exported, "bar");
    assert_eq!(result[1].local, "$abc$bar");
    assert!(result[1].is_weak);
    assert!(!result[1].is_esm_export);
    assert!(!result[1].is_static_binding_safe);
  }

  #[test]
  fn test_deserialize_symbols_field_map_format() {
    // JS Map serialization format: object with exported names as keys
    // This is how JS `Map<exported, {local, loc, isWeak, meta}>` serializes
    let value = serde_json::json!({
      "foo": {
        "local": "$abc$foo",
        "loc": null,
        "isWeak": false,
        "meta": {
          "isEsm": true,
          "isStaticBindingSafe": true
        }
      },
      "bar": {
        "local": "$abc$bar",
        "loc": null,
        "isWeak": true,
        "meta": {
          "isEsm": false,
          "isStaticBindingSafe": false
        }
      }
    });

    let result = deserialize_symbols_field(value).unwrap().unwrap();

    assert_eq!(result.len(), 2);

    // Find foo and bar (order not guaranteed in JSON object)
    let foo = result.iter().find(|s| s.exported == "foo").unwrap();
    let bar = result.iter().find(|s| s.exported == "bar").unwrap();

    assert_eq!(foo.local, "$abc$foo");
    assert!(!foo.is_weak);
    assert!(foo.is_esm_export);
    assert!(foo.is_static_binding_safe);

    assert_eq!(bar.local, "$abc$bar");
    assert!(bar.is_weak);
    assert!(!bar.is_esm_export);
    assert!(!bar.is_static_binding_safe);
  }

  #[test]
  fn test_deserialize_symbols_field_map_format_missing_meta() {
    // JS Map format with missing meta fields (should use defaults)
    let value = serde_json::json!({
      "foo": {
        "local": "$abc$foo",
        "isWeak": true
      }
    });

    let result = deserialize_symbols_field(value).unwrap().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].exported, "foo");
    assert_eq!(result[0].local, "$abc$foo");
    assert!(result[0].is_weak);
    // Defaults when meta is missing
    assert!(!result[0].is_esm_export);
    assert!(!result[0].is_static_binding_safe);
    assert!(!result[0].self_referenced);
  }

  #[test]
  fn test_deserialize_symbols_field_empty_array() {
    let value = serde_json::json!([]);
    let result = deserialize_symbols_field(value).unwrap().unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn test_deserialize_symbols_field_empty_object() {
    let value = serde_json::json!({});
    let result = deserialize_symbols_field(value).unwrap().unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn test_deserialize_symbols_field_invalid_type() {
    let value = serde_json::json!("not an array or object");
    let result = deserialize_symbols_field(value);
    assert!(result.is_err());
    assert_eq!(
      result.unwrap_err(),
      "symbols must be an array, object, or null"
    );
  }
}
