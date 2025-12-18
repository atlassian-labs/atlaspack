use serde::de::{Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::json;

use crate::types::Asset;
use crate::types::serialization::extract_asset_meta_fields;

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
    let mut output_hash = None;
    let mut config_path = None;
    let mut config_key_path = None;
    let mut unique_key = None;
    let mut meta = None;

    while let Some(key) = map.next_key::<String>()? {
      match key.as_str() {
        "bundleBehavior" => bundle_behavior = map.next_value()?,
        "configKeyPath" => config_key_path = map.next_value()?,
        "configPath" => config_path = map.next_value()?,
        "env" => env = Some(map.next_value()?),
        "filePath" => file_path = Some(map.next_value()?),
        "id" => id = Some(map.next_value()?),
        "isBundleSplittable" => is_bundle_splittable = Some(map.next_value()?),
        "isSource" => is_source = Some(map.next_value()?),
        "outputHash" => output_hash = map.next_value()?,
        "pipeline" => pipeline = map.next_value()?,
        "query" => query = map.next_value()?,
        "sideEffects" => side_effects = Some(map.next_value()?),
        "stats" => stats = Some(map.next_value()?),
        "symbols" => symbols = map.next_value()?,
        "type" => file_type = Some(map.next_value()?),
        "uniqueKey" => unique_key = map.next_value()?,
        "meta" => {
          let meta_map: serde_json::Value = map.next_value()?;
          meta = Some(
            meta_map
              .as_object()
              .unwrap_or(&serde_json::Map::new())
              .clone(),
          );
        }
        _ => {
          return Err(serde::de::Error::unknown_field(&key, &[]));
        }
      }
    }

    let mut meta_map = meta.unwrap_or_default();
    let extracted = extract_asset_meta_fields(&mut meta_map);

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
      has_cjs_exports: extracted.has_cjs_exports,
      output_hash,
      config_path,
      config_key_path,
      unique_key,
      meta: meta_map,
      conditions: extracted.conditions,
      has_node_replacements: extracted.has_node_replacements,
      inline_type: extracted.inline_type,
      is_constant_module: extracted.is_constant_module,
      should_wrap: extracted.should_wrap,
      static_exports: extracted.static_exports,
      css_dependency_type: extracted.css_dependency_type,
      empty_file_star_reexport: extracted.empty_file_star_reexport,
      has_dependencies: extracted.has_dependencies,
      has_references: extracted.has_references,
      packaging_id: extracted.packaging_id,
      interpreter: extracted.interpreter,
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
  use crate::types::asset::AssetStats;
  use crate::types::json::JSONObject;
  use crate::types::{Environment, FileType};
  use pretty_assertions::assert_eq;
  use std::collections::BTreeSet;
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
      code: Default::default(),
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
      conditions: BTreeSet::new(),
      config_path: Some("config.json".to_string()),
      config_key_path: Some("key.path".to_string()),
      interpreter: Some("#!/usr/bin/node".to_string()),
      packaging_id: Some("pkg123".to_string()),
      has_references: Some(true),
      css_dependency_type: Some("tag".to_string()),
      inline_type: Some("string".to_string()),
      empty_file_star_reexport: Some(false),
      has_dependencies: Some(true),
    };

    // Test serialization
    let serialized = serde_json::to_string(&asset).expect("Failed to serialize");

    // Test deserialization
    let deserialized: Asset = serde_json::from_str(&serialized).expect("Failed to deserialize");

    // Compare fields that survive serialization/deserialization
    assert_eq!(asset, deserialized);
  }

  #[test]
  fn test_asset_serialization_basic() {
    // Test basic serialization functionality
    let asset = Asset {
      id: "basic_asset".to_string(),
      file_path: PathBuf::from("/path/to/basic.js"),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      is_source: true,
      side_effects: false,
      is_bundle_splittable: true,
      empty_file_star_reexport: Some(true),
      ..Asset::default()
    };

    let serialized = serde_json::to_string(&asset).expect("Failed to serialize");
    let parsed: serde_json::Value =
      serde_json::from_str(&serialized).expect("Failed to parse JSON");

    let expected = serde_json::json!({
      "bundleBehavior": null,
      "configKeyPath": null,
      "configPath": null,
      "env": {
        "context": "browser",
        "engines": {
          "browsers": null
        },
        "includeNodeModules": true,
        "isLibrary": false,
        "loc": null,
        "outputFormat": "global",
        "shouldScopeHoist": false,
        "shouldOptimize": false,
        "sourceMap": null,
        "sourceType": "module",
        "unstableSingleFileOutput": false
      },
      "filePath": "/path/to/basic.js",
      "id": "basic_asset",
      "isBundleSplittable": true,
      "isSource": true,
      "outputHash": null,
      "pipeline": null,
      "query": null,
      "sideEffects": false,
      "stats": {
        "size": 0,
        "time": 0
      },
      "symbols": null,
      "type": "js",
      "meta": {
        "conditions": [],
        "hasCJSExports": false,
        "has_node_replacements": false,
        "inlineType": null,
        "isConstantModule": false,
        "shouldWrap": false,
        "staticExports": false,
        "type": null,
        "emptyFileStarReexport": true,
      }
    });

    assert_eq!(expected, parsed);
  }

  #[test]
  fn test_asset_meta_field_serialization() {
    // Test that meta fields are properly serialized
    let mut meta = JSONObject::new();
    meta.insert("customField".to_string(), serde_json::json!("customValue"));
    meta.insert("numField".to_string(), serde_json::json!(42));

    let asset = Asset {
      id: "meta_test_asset".to_string(),
      file_path: PathBuf::from("/test/meta.js"),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      meta: meta.clone(),
      unique_key: Some("meta_unique".to_string()),
      pipeline: Some("babel".to_string()),
      has_cjs_exports: true,
      should_wrap: false,
      inline_type: Some("string".to_string()),
      packaging_id: Some("1234".to_string()),
      ..Asset::default()
    };

    let json = serde_json::to_string_pretty(&asset).expect("Failed to serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    let expected = serde_json::json!({
      "bundleBehavior": null,
      "configKeyPath": null,
      "configPath": null,
      "env": {
        "context": "browser",
        "engines": {
          "browsers": null
        },
        "includeNodeModules": true,
        "isLibrary": false,
        "loc": null,
        "outputFormat": "global",
        "shouldScopeHoist": false,
        "shouldOptimize": false,
        "sourceMap": null,
        "sourceType": "module",
        "unstableSingleFileOutput": false
      },
      "filePath": "/test/meta.js",
      "id": "meta_test_asset",
      "isBundleSplittable": false,
      "isSource": false,
      "outputHash": null,
      "pipeline": "babel",
      "query": null,
      "sideEffects": false,
      "stats": {
        "size": 0,
        "time": 0
      },
      "symbols": null,
      "type": "js",
      "uniqueKey": "meta_unique",
      "meta": {
        "id": "1234",
        "customField": "customValue",
        "numField": 42,
        "conditions": [],
        "hasCJSExports": true,
        "has_node_replacements": false,
        "inlineType": "string",
        "isConstantModule": false,
        "shouldWrap": false,
        "staticExports": false,
        "type": null
      }
    });

    assert_eq!(expected, parsed);
  }

  #[test]
  fn test_asset_serialization_with_optional_fields() {
    // Test serialization with various optional fields
    let asset = Asset {
      id: "optional_fields_asset".to_string(),
      file_path: PathBuf::from("/test/optional.js"),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      unique_key: Some("optional_unique".to_string()),
      pipeline: Some("typescript".to_string()),
      query: Some("?optional=true".to_string()),
      config_path: Some("/config.json".to_string()),
      config_key_path: Some("ts.compile".to_string()),
      output_hash: Some("optional_hash".to_string()),
      ..Asset::default()
    };

    let json = serde_json::to_string(&asset).expect("Failed to serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    let expected = serde_json::json!({
      "bundleBehavior": null,
      "configKeyPath": "ts.compile",
      "configPath": "/config.json",
      "env": {
        "context": "browser",
        "engines": {
          "browsers": null
        },
        "includeNodeModules": true,
        "isLibrary": false,
        "loc": null,
        "outputFormat": "global",
        "shouldScopeHoist": false,
        "shouldOptimize": false,
        "sourceMap": null,
        "sourceType": "module",
        "unstableSingleFileOutput": false
      },
      "filePath": "/test/optional.js",
      "id": "optional_fields_asset",
      "isBundleSplittable": false,
      "isSource": false,
      "outputHash": "optional_hash",
      "pipeline": "typescript",
      "query": "?optional=true",
      "sideEffects": false,
      "stats": {
        "size": 0,
        "time": 0
      },
      "symbols": null,
      "type": "js",
      "uniqueKey": "optional_unique",
      "meta": {
        "conditions": [],
        "hasCJSExports": false,
        "has_node_replacements": false,
        "inlineType": null,
        "isConstantModule": false,
        "shouldWrap": false,
        "staticExports": false,
        "type": null
      }
    });

    assert_eq!(expected, parsed);
  }
}
