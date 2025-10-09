use serde::de::{Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::json;

use crate::types::Asset;

macro_rules! insert_if_not_none {
  ($map:expr, $key:expr, $value:expr) => {
    if let Some(v) = $value {
      $map.insert($key.to_string(), json!(v));
    }
  };
}

macro_rules! insert_json {
  ($map:expr, $key:expr, $value:expr) => {
    $map.insert($key.to_string(), json!($value));
  };
}

impl Serialize for Asset {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut state = serializer.serialize_struct("Asset", 5)?;
    state.serialize_field("bundleBehavior", &self.bundle_behavior)?;
    state.serialize_field("configKeyPath", &self.config_key_path)?;
    state.serialize_field("configPath", &self.config_path)?;
    state.serialize_field("env", &self.env)?;
    state.serialize_field("filePath", &self.file_path)?;
    state.serialize_field("hasCJSExports", &self.has_cjs_exports)?;
    state.serialize_field("id", &self.id)?;
    state.serialize_field("isBundleSplittable", &self.is_bundle_splittable)?;
    state.serialize_field("isSource", &self.is_source)?;
    state.serialize_field("outputHash", &self.output_hash)?;
    state.serialize_field("pipeline", &self.pipeline)?;
    state.serialize_field("query", &self.query)?;
    state.serialize_field("sideEffects", &self.side_effects)?;
    state.serialize_field("stats", &self.stats)?;
    state.serialize_field("symbols", &self.symbols)?;
    state.serialize_field("type", &self.file_type)?;

    if let Some(unique_key) = &self.unique_key {
      state.serialize_field("uniqueKey", unique_key)?;
    }

    let mut meta = self.meta.clone();

    insert_json!(meta, "conditions", &self.conditions);
    insert_json!(meta, "hasCJSExports", &self.has_cjs_exports);
    insert_json!(meta, "has_node_replacements", &self.has_node_replacements);
    insert_json!(meta, "inlineType", &self.inline_type);
    insert_json!(meta, "isConstantModule", &self.is_constant_module);
    insert_json!(meta, "shouldWrap", &self.should_wrap);
    insert_json!(meta, "staticExports", &self.static_exports);
    insert_json!(meta, "type", &self.css_dependency_type);

    insert_if_not_none!(
      meta,
      "emptyFileStarReexport",
      &self.empty_file_star_reexport
    );
    insert_if_not_none!(meta, "hasDependencies", &self.has_dependencies);
    insert_if_not_none!(meta, "hasReferences", &self.has_references);
    insert_if_not_none!(meta, "id", &self.packaging_id);
    insert_if_not_none!(meta, "interpreter", &self.interpreter);

    state.serialize_field("meta", &meta)?;
    state.end()
  }
}

impl<'de> Deserialize<'de> for Asset {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_map(AssetVisitor)
  }
}

struct AssetVisitor;

impl<'de> Visitor<'de> for AssetVisitor {
  type Value = Asset;

  fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
  where
    A: serde::de::MapAccess<'de>,
  {
    let mut id = None;
    let mut bundle_behavior = None;
    let mut env = None;
    let mut file_path = None;
    let mut file_type = None;
    let mut pipeline = None;
    let mut query = None;
    let mut stats = None;
    let mut symbols = None;
    let mut side_effects = None;
    let mut is_bundle_splittable = None;
    let mut is_source = None;
    let mut has_cjs_exports = None;
    let mut output_hash = None;
    let mut config_path = None;
    let mut config_key_path = None;
    let mut unique_key = None;
    let mut meta = None;

    // Fields extracted from meta
    let mut conditions = None;
    let mut has_node_replacements = None;
    let mut inline_type = None;
    let mut is_constant_module = None;
    let mut should_wrap = None;
    let mut static_exports = None;
    let mut css_dependency_type = None;
    let mut empty_file_star_reexport = None;
    let mut has_dependencies = None;
    let mut has_references = None;
    let mut packaging_id = None;
    let mut interpreter = None;

    while let Some(key) = map.next_key::<String>()? {
      match key.as_str() {
        "bundleBehavior" => bundle_behavior = map.next_value()?,
        "configKeyPath" => config_key_path = Some(map.next_value()?),
        "configPath" => config_path = Some(map.next_value()?),
        "env" => env = Some(map.next_value()?),
        "filePath" => file_path = Some(map.next_value()?),
        "hasCJSExports" => has_cjs_exports = Some(map.next_value()?),
        "id" => id = Some(map.next_value()?),
        "isBundleSplittable" => is_bundle_splittable = Some(map.next_value()?),
        "isSource" => is_source = Some(map.next_value()?),
        "outputHash" => output_hash = Some(map.next_value()?),
        "pipeline" => pipeline = Some(map.next_value()?),
        "query" => query = Some(map.next_value()?),
        "sideEffects" => side_effects = Some(map.next_value()?),
        "stats" => stats = Some(map.next_value()?),
        "symbols" => symbols = Some(map.next_value()?),
        "type" => file_type = Some(map.next_value()?),
        "uniqueKey" => unique_key = Some(map.next_value()?),
        "meta" => {
          let meta_value: serde_json::Value = map.next_value()?;
          if let Some(meta_obj) = meta_value.as_object() {
            // Extract specific fields from meta
            if let Some(conditions_val) = meta_obj.get("conditions") {
              conditions = Some(
                serde_json::from_value(conditions_val.clone()).map_err(serde::de::Error::custom)?,
              );
            }

            if let Some(has_cjs_exports_val) = meta_obj.get("hasCJSExports")
              && let Some(val) = has_cjs_exports_val.as_bool()
            {
              has_cjs_exports = Some(val);
            }

            if let Some(has_node_replacements_val) = meta_obj.get("has_node_replacements")
              && let Some(val) = has_node_replacements_val.as_bool()
            {
              has_node_replacements = Some(val);
            }

            if let Some(inline_type_val) = meta_obj.get("inlineType")
              && !inline_type_val.is_null()
            {
              inline_type = Some(
                serde_json::from_value(inline_type_val.clone())
                  .map_err(serde::de::Error::custom)?,
              );
            }

            if let Some(is_constant_module_val) = meta_obj.get("isConstantModule")
              && let Some(val) = is_constant_module_val.as_bool()
            {
              is_constant_module = Some(val);
            }

            if let Some(should_wrap_val) = meta_obj.get("shouldWrap")
              && let Some(val) = should_wrap_val.as_bool()
            {
              should_wrap = Some(val);
            }

            if let Some(static_exports_val) = meta_obj.get("staticExports")
              && let Some(val) = static_exports_val.as_bool()
            {
              static_exports = Some(val);
            }

            if let Some(css_type_val) = meta_obj.get("type")
              && !css_type_val.is_null()
            {
              css_dependency_type = Some(
                serde_json::from_value(css_type_val.clone()).map_err(serde::de::Error::custom)?,
              );
            }

            if let Some(empty_star_val) = meta_obj.get("emptyFileStarReexport")
              && let Some(val) = empty_star_val.as_bool()
            {
              empty_file_star_reexport = Some(val);
            }

            if let Some(has_deps_val) = meta_obj.get("hasDependencies")
              && let Some(val) = has_deps_val.as_bool()
            {
              has_dependencies = Some(val);
            }

            if let Some(has_refs_val) = meta_obj.get("hasReferences")
              && let Some(val) = has_refs_val.as_bool()
            {
              has_references = Some(val);
            }

            if let Some(pkg_id_val) = meta_obj.get("id")
              && let Some(val) = pkg_id_val.as_str()
            {
              packaging_id = Some(val.to_string());
            }

            if let Some(interpreter_val) = meta_obj.get("interpreter")
              && let Some(val) = interpreter_val.as_str()
            {
              interpreter = Some(val.to_string());
            }
          }
          meta = Some(serde_json::from_value(meta_value).map_err(serde::de::Error::custom)?);
        }
        _ => {
          // Skip unknown fields instead of erroring
          let _: serde_json::Value = map.next_value()?;
        }
      }
    }

    Ok(Asset {
      id: id.ok_or_else(|| serde::de::Error::missing_field("id"))?,
      bundle_behavior,
      env: env.unwrap_or_default(),
      file_path: file_path.unwrap_or_default(),
      file_type: file_type.ok_or_else(|| serde::de::Error::missing_field("type"))?,
      pipeline,
      query,
      stats: stats.unwrap_or_default(),
      symbols,
      side_effects: side_effects.unwrap_or_default(),
      is_bundle_splittable: is_bundle_splittable.unwrap_or(true),
      is_source: is_source.unwrap_or_default(),
      has_cjs_exports: has_cjs_exports.unwrap_or_default(),
      output_hash,
      config_path,
      config_key_path,
      unique_key,
      meta: meta.unwrap_or_default(),
      conditions: conditions.unwrap_or_default(),
      has_node_replacements: has_node_replacements.unwrap_or_default(),
      inline_type: inline_type.unwrap_or_default(),
      is_constant_module: is_constant_module.unwrap_or_default(),
      should_wrap: should_wrap.unwrap_or_default(),
      static_exports: static_exports.unwrap_or_default(),
      css_dependency_type: css_dependency_type.unwrap_or_default(),
      empty_file_star_reexport,
      has_dependencies,
      has_references,
      packaging_id,
      interpreter,
      ..Asset::default()
    })
  }

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("An Asset")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::asset::{AssetStats, Code};
  use crate::types::json::JSONObject;
  use crate::types::{Environment, FileType};
  use std::collections::HashSet;
  use std::path::PathBuf;
  use std::sync::Arc;

  #[test]
  fn test_asset_serialize_deserialize() {
    // Create a test asset with various fields set
    let asset = Asset {
      id: "test123".to_string(),
      bundle_behavior: Default::default(),
      env: Arc::new(Environment::default()),
      file_path: PathBuf::from("test.js"),
      file_type: FileType::Js,
      code: Code::from("console.log('hello')"),
      map: None,
      meta: JSONObject::default(),
      pipeline: Some("js".to_string()),
      query: Some("?inline".to_string()),
      stats: AssetStats {
        size: 100,
        time: 50,
      },
      symbols: Some(vec![]),
      unique_key: Some("unique123".to_string()),
      side_effects: true,
      is_bundle_splittable: false,
      is_source: true,
      is_virtual: false,
      has_cjs_exports: true,
      output_hash: Some("hash123".to_string()),
      static_exports: true,
      should_wrap: true,
      has_node_replacements: true,
      is_constant_module: true,
      conditions: HashSet::new(),
      config_path: Some("config.json".to_string()),
      config_key_path: Some("key.path".to_string()),
      interpreter: Some("#!/usr/bin/node".to_string()),
      packaging_id: Some("pkg123".to_string()),
      has_references: Some(true),
      css_dependency_type: Default::default(),
      inline_type: Default::default(),
      empty_file_star_reexport: Some(false),
      has_dependencies: Some(true),
    };

    // Test serialization
    let serialized = serde_json::to_string(&asset).expect("Failed to serialize");

    // Test deserialization
    let deserialized: Asset = serde_json::from_str(&serialized).expect("Failed to deserialize");

    // Basic checks
    assert_eq!(asset.id, deserialized.id);
    assert_eq!(asset.file_type, deserialized.file_type);
    assert_eq!(asset.has_cjs_exports, deserialized.has_cjs_exports);
    assert_eq!(asset.should_wrap, deserialized.should_wrap);
    assert_eq!(asset.static_exports, deserialized.static_exports);
    assert_eq!(asset.is_constant_module, deserialized.is_constant_module);
    assert_eq!(
      asset.has_node_replacements,
      deserialized.has_node_replacements
    );
    assert_eq!(asset.interpreter, deserialized.interpreter);
    assert_eq!(asset.packaging_id, deserialized.packaging_id);
    assert_eq!(asset.has_references, deserialized.has_references);
    assert_eq!(
      asset.empty_file_star_reexport,
      deserialized.empty_file_star_reexport
    );
    assert_eq!(asset.has_dependencies, deserialized.has_dependencies);
    assert_eq!(asset.pipeline, deserialized.pipeline);
    assert_eq!(asset.query, deserialized.query);
    assert_eq!(asset.unique_key, deserialized.unique_key);
    assert_eq!(asset.config_path, deserialized.config_path);
    assert_eq!(asset.config_key_path, deserialized.config_key_path);
    assert_eq!(asset.side_effects, deserialized.side_effects);
    assert_eq!(
      asset.is_bundle_splittable,
      deserialized.is_bundle_splittable
    );
    assert_eq!(asset.is_source, deserialized.is_source);
    assert_eq!(asset.output_hash, deserialized.output_hash);
  }
}
