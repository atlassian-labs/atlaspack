use std::collections::BTreeMap;

use crate::types::{Dependency, DependencyKind};
use serde::de::{Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

impl Serialize for Dependency {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut state = serializer.serialize_struct("Dependency", 25)?;

    // Core fields
    state.serialize_field("id", &self.id)?;
    state.serialize_field("bundleBehavior", &self.bundle_behavior)?;
    state.serialize_field("env", &self.env)?;
    state.serialize_field("specifier", &self.specifier)?;
    state.serialize_field("specifierType", &self.specifier_type)?;
    state.serialize_field("priority", &self.priority)?;

    // Optional core fields
    if let Some(ref loc) = self.loc {
      state.serialize_field("loc", loc)?;
    }
    if let Some(ref pipeline) = self.pipeline {
      state.serialize_field("pipeline", pipeline)?;
    }
    if let Some(ref range) = self.range {
      state.serialize_field("range", range)?;
    }
    if let Some(ref resolve_from) = self.resolve_from {
      state.serialize_field("resolveFrom", resolve_from)?;
    }
    if let Some(ref source_asset_id) = self.source_asset_id {
      state.serialize_field("sourceAssetId", source_asset_id)?;
    }
    if let Some(ref source_path) = self.source_path {
      state.serialize_field("sourcePath", source_path)?;
    }
    if let Some(ref source_asset_type) = self.source_asset_type {
      state.serialize_field("sourceAssetType", source_asset_type)?;
    }
    if let Some(ref symbols) = self.symbols {
      state.serialize_field("symbols", symbols)?;
    }
    if let Some(ref target) = self.target {
      state.serialize_field("target", target)?;
    }

    // Top-level booleans that remain
    state.serialize_field("isEntry", &self.is_entry)?;
    state.serialize_field("isOptional", &self.is_optional)?;

    // Package conditions (serialize only if not empty)
    if !self.package_conditions.is_empty() {
      state.serialize_field("packageConditions", &self.package_conditions)?;
    }

    // Build meta to include selected fields
    let mut meta = self.meta.clone();

    if let Some(ref kind) = self.kind {
      meta.insert("kind".to_string(), serde_json::json!(kind));
    }
    if let Some(ref promise_symbol) = self.promise_symbol {
      meta.insert(
        "promiseSymbol".to_string(),
        serde_json::json!(promise_symbol),
      );
    }
    // Add import_attributes to meta if not empty
    if !self.import_attributes.is_empty() {
      meta.insert(
        "importAttributes".to_string(),
        serde_json::to_value(&self.import_attributes).unwrap(),
      );
    }
    if let Some(ref placeholder) = self.placeholder {
      meta.insert("placeholder".to_string(), serde_json::json!(placeholder));
    }
    if let Some(ref media) = self.media {
      meta.insert("media".to_string(), serde_json::json!(media));
    }
    if let Some(ref chunk_name) = self.chunk_name_magic_comment {
      meta.insert(
        "chunkNameMagicComment".to_string(),
        serde_json::json!(chunk_name),
      );
    }
    // Always include these booleans in meta
    meta.insert(
      "needsStableName".to_string(),
      serde_json::json!(self.needs_stable_name),
    );
    meta.insert(
      "shouldWrap".to_string(),
      serde_json::json!(self.should_wrap),
    );
    meta.insert("isEsm".to_string(), serde_json::json!(self.is_esm));
    meta.insert(
      "webworker".to_string(),
      serde_json::json!(self.is_webworker),
    );
    meta.insert(
      "isCssImport".to_string(),
      serde_json::json!(self.is_css_import),
    );

    // Serialize meta field
    state.serialize_field("meta", &meta)?;

    state.end()
  }
}

impl<'de> Deserialize<'de> for Dependency {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_map(DependencyVisitor)
  }
}

struct DependencyVisitor;

impl<'de> Visitor<'de> for DependencyVisitor {
  type Value = Dependency;

  fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
  where
    A: serde::de::MapAccess<'de>,
  {
    let mut id = None;
    let mut bundle_behavior = None;
    let mut env = None;
    let mut loc = None;
    let mut meta = None;
    let mut package_conditions = None;
    let mut pipeline = None;
    let mut priority = None;
    let mut range = None;
    let mut resolve_from = None;
    let mut source_asset_id = None;
    let mut source_path = None;
    let mut specifier = None;
    let mut specifier_type = None;
    let mut source_asset_type = None;
    let mut symbols = None;
    let mut target = None;
    let mut is_entry = None;
    let mut is_optional = None;

    while let Some(key) = map.next_key::<String>()? {
      match key.as_str() {
        "id" => id = Some(map.next_value()?),
        "bundleBehavior" => bundle_behavior = Some(map.next_value()?),
        "env" => env = Some(map.next_value()?),
        "loc" => loc = Some(map.next_value()?),
        "meta" => {
          let meta_map: serde_json::Value = map.next_value()?;
          meta = Some(
            meta_map
              .as_object()
              .unwrap_or(&serde_json::Map::new())
              .clone(),
          );
        }
        "packageConditions" => package_conditions = Some(map.next_value()?),
        "pipeline" => pipeline = Some(map.next_value()?),
        "priority" => priority = Some(map.next_value()?),
        "range" => range = Some(map.next_value()?),
        "resolveFrom" => resolve_from = Some(map.next_value()?),
        "sourceAssetId" => source_asset_id = Some(map.next_value()?),
        "sourcePath" => source_path = Some(map.next_value()?),
        "specifier" => specifier = Some(map.next_value()?),
        "specifierType" => specifier_type = Some(map.next_value()?),
        "sourceAssetType" => source_asset_type = Some(map.next_value()?),
        "symbols" => symbols = Some(map.next_value()?),
        "target" => target = Some(map.next_value()?),
        "isEntry" => is_entry = Some(map.next_value()?),
        "isOptional" => is_optional = Some(map.next_value()?),
        _ => {
          // Skip unknown fields instead of erroring
          let _: serde_json::Value = map.next_value()?;
        }
      }
    }

    // Extract fields from meta if not present at top level
    let mut meta_map = meta.unwrap_or_default();

    // Extract fields ONLY from meta - these should never be at top level
    let final_needs_stable_name = meta_map
      .get("needsStableName")
      .and_then(|v| v.as_bool())
      .unwrap_or_default();

    let final_should_wrap = meta_map
      .get("shouldWrap")
      .and_then(|v| v.as_bool())
      .unwrap_or_default();

    let final_is_esm = meta_map
      .get("isEsm")
      .and_then(|v| v.as_bool())
      .unwrap_or_default();

    let final_is_webworker = meta_map
      .get("webworker")
      .and_then(|v| v.as_bool())
      .unwrap_or_default();

    let final_is_css_import = meta_map
      .get("isCssImport")
      .and_then(|v| v.as_bool())
      .unwrap_or_default();

    let final_placeholder = meta_map
      .get("placeholder")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());

    let final_media = meta_map
      .get("media")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());

    let final_chunk_name_magic_comment = meta_map
      .get("chunkNameMagicComment")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());

    let final_import_attributes = meta_map
      .get("importAttributes")
      .and_then(|v| serde_json::from_value(v.clone()).ok())
      .unwrap_or_else(BTreeMap::default);

    let kind = meta_map
      .get("kind")
      .and_then(|v| DependencyKind::deserialize(v).ok());

    let promise_symbol = meta_map
      .get("promiseSymbol")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());

    // Remove the extracted fields from meta to maintain symmetry
    meta_map.remove("needsStableName");
    meta_map.remove("shouldWrap");
    meta_map.remove("isEsm");
    meta_map.remove("webworker");
    meta_map.remove("isCssImport");
    meta_map.remove("placeholder");
    meta_map.remove("media");
    meta_map.remove("chunkNameMagicComment");
    meta_map.remove("importAttributes");
    meta_map.remove("kind");
    meta_map.remove("promiseSymbol");

    Ok(Dependency {
      id: id.ok_or_else(|| serde::de::Error::missing_field("id"))?,
      bundle_behavior: bundle_behavior.unwrap_or_default(),
      env: env.ok_or_else(|| serde::de::Error::missing_field("env"))?,
      loc,
      meta: meta_map,
      package_conditions: package_conditions.unwrap_or_default(),
      pipeline,
      priority: priority.ok_or_else(|| serde::de::Error::missing_field("priority"))?,
      range,
      resolve_from,
      source_asset_id,
      source_path,
      specifier: specifier.ok_or_else(|| serde::de::Error::missing_field("specifier"))?,
      specifier_type: specifier_type
        .ok_or_else(|| serde::de::Error::missing_field("specifierType"))?,
      source_asset_type,
      symbols,
      target,
      is_entry: is_entry.unwrap_or_default(),
      is_optional: is_optional.unwrap_or_default(),
      needs_stable_name: final_needs_stable_name,
      should_wrap: final_should_wrap,
      is_esm: final_is_esm,
      placeholder: final_placeholder,
      is_webworker: final_is_webworker,
      kind,
      promise_symbol,
      import_attributes: final_import_attributes,
      media: final_media,
      is_css_import: final_is_css_import,
      chunk_name_magic_comment: final_chunk_name_magic_comment,
    })
  }

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("A Dependency")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{
    Environment, EnvironmentContext, IncludeNodeModules, OutputFormat, Priority, SourceType,
    SpecifierType,
  };
  use pretty_assertions::assert_eq;
  use std::collections::BTreeMap;
  use std::path::PathBuf;
  use std::sync::Arc;

  #[test]
  fn test_dependency_serialize_deserialize() {
    // Create a test dependency with various fields set
    let mut import_attributes = BTreeMap::new();
    import_attributes.insert("type".to_string(), true);

    let dependency = Dependency {
      id: "dep123".to_string(),
      bundle_behavior: Default::default(),
      env: Arc::new(Environment::default()),
      specifier: "./test.js".to_string(),
      specifier_type: SpecifierType::Esm,
      priority: Priority::Sync,
      loc: None,
      meta: Default::default(),
      package_conditions: Default::default(),
      pipeline: Some("js".to_string()),
      range: Some("^1.0.0".to_string()),
      resolve_from: Some(PathBuf::from("/src")),
      source_asset_id: Some("asset123".to_string()),
      source_path: Some(PathBuf::from("index.js")),
      source_asset_type: None,
      symbols: None,
      target: None,
      is_entry: false,
      is_optional: true,
      needs_stable_name: false,
      should_wrap: true,
      is_esm: true,
      placeholder: Some("PLACEHOLDER".to_string()),
      is_webworker: false,
      kind: None,
      promise_symbol: Some("import123".to_string()),
      import_attributes,
      media: Some("screen".to_string()),
      is_css_import: false,
      chunk_name_magic_comment: Some("my-chunk".to_string()),
    };

    // Test serialization
    let serialized = serde_json::to_string(&dependency).expect("Failed to serialize");

    // Test deserialization
    let deserialized: Dependency =
      serde_json::from_str(&serialized).expect("Failed to deserialize");

    // Assert the entire struct matches
    assert_eq!(dependency, deserialized);
  }

  #[test]
  fn test_dependency_minimal_serialize_deserialize() {
    // Test with minimal required fields
    let dependency = Dependency {
      id: "minimal_dep".to_string(),
      env: Arc::new(Environment::default()),
      specifier: "./minimal.js".to_string(),
      specifier_type: SpecifierType::Esm,
      priority: Priority::Sync,
      ..Default::default()
    };

    let serialized = serde_json::to_string(&dependency).expect("Failed to serialize");
    let deserialized: Dependency =
      serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(dependency, deserialized);
  }

  #[test]
  fn test_dependency_json_output_format() {
    // Test that JSON output has the expected structure and exact ordering

    let dependency = Dependency {
      id: "test_json_dep".to_string(),
      env: Arc::new(Environment::default()),
      specifier: "./test.js".to_string(),
      specifier_type: SpecifierType::Esm,
      priority: Priority::Lazy,
      pipeline: Some("typescript".to_string()),
      range: Some("^2.0.0".to_string()),
      is_entry: true,
      is_optional: false,
      needs_stable_name: true,
      should_wrap: false,
      is_esm: true,
      is_webworker: false,
      is_css_import: false,
      placeholder: Some("TEST_PLACEHOLDER".to_string()),
      media: Some("(min-width: 800px)".to_string()),
      chunk_name_magic_comment: Some("my-dynamic-chunk".to_string()),
      kind: Some(DependencyKind::Import),
      promise_symbol: Some("symbol".to_string()),
      ..Default::default()
    };

    let json_output = serde_json::to_string_pretty(&dependency).expect("Failed to serialize");

    // Build expected JSON value in a single expression (insertion order matches serializer)
    let env_value = serde_json::to_value(Environment::default()).unwrap();
    let expected = serde_json::json!({
      "id": "test_json_dep",
      "bundleBehavior": null,
      "env": env_value,
      "specifier": "./test.js",
      "specifierType": 0,
      "priority": 2,
      "pipeline": "typescript",
      "range": "^2.0.0",
      "isEntry": true,
      "isOptional": false,
      "meta": {
        "kind": "Import",
        "promiseSymbol": "symbol",
        "placeholder": "TEST_PLACEHOLDER",
        "media": "(min-width: 800px)",
        "chunkNameMagicComment": "my-dynamic-chunk",
        "needsStableName": true,
        "shouldWrap": false,
        "isEsm": true,
        "webworker": false,
        "isCssImport": false
      }
    });

    let expected_string = serde_json::to_string_pretty(&expected).unwrap();

    assert_eq!(json_output, expected_string);
  }

  #[test]
  fn test_dependency_deserialize_from_json_string() {
    // Test deserializing from a hand-crafted JSON string
    let json_input = r#"{
      "id": "json_string_dep",
      "bundleBehavior": null,
      "env": {
        "context": "browser",
        "engines": {},
        "includeNodeModules": false,
        "outputFormat": "global",
        "isLibrary": false,
        "shouldOptimize": false,
        "shouldScopeHoist": false,
        "sourceMap": null,
        "sourceType": "module",
        "unstableSingleFileOutput": false
      },
      "specifier": "./from-json.js",
      "specifierType": 1,
      "priority": 0,
      "pipeline": "babel",
      "range": "~1.5.0",
      "isEntry": false,
      "isOptional": true,
      "meta": {
        "customField": "customValue",
        "needsStableName": false,
        "shouldWrap": true,
        "isEsm": false,
        "webworker": true,
        "isCssImport": true,
        "placeholder": "JSON_PLACEHOLDER",
        "media": "print",
        "chunkNameMagicComment": "json-chunk",
        "importAttributes": {
          "type": true,
          "assert": false
        }
      }
    }"#;

    let deserialized: Dependency =
      serde_json::from_str(json_input).expect("Failed to deserialize from JSON");

    // Build expected dependency struct
    let mut expected_import_attributes = BTreeMap::new();
    expected_import_attributes.insert("type".to_string(), true);
    expected_import_attributes.insert("assert".to_string(), false);

    let mut expected_meta = crate::types::JSONObject::new();
    expected_meta.insert("customField".to_string(), serde_json::json!("customValue"));

    let expected = Dependency {
      id: "json_string_dep".to_string(),
      bundle_behavior: Default::default(),
      env: Arc::new(Environment {
        context: EnvironmentContext::Browser,
        engines: Default::default(),
        include_node_modules: IncludeNodeModules::Bool(false),
        output_format: OutputFormat::Global,
        is_library: false,
        should_optimize: false,
        should_scope_hoist: false,
        source_map: None,
        source_type: SourceType::Module,
        unstable_single_file_output: false,
        custom_env: None,
        loc: None,
      }),
      specifier: "./from-json.js".to_string(),
      specifier_type: SpecifierType::CommonJS, // 1 = CommonJS
      priority: Priority::Sync,                // 0 = Sync
      loc: None,
      meta: expected_meta,
      package_conditions: Default::default(),
      pipeline: Some("babel".to_string()),
      range: Some("~1.5.0".to_string()),
      resolve_from: None,
      source_asset_id: None,
      source_path: None,
      source_asset_type: None,
      symbols: None,
      target: None,
      is_entry: false,
      is_optional: true,
      needs_stable_name: false,
      should_wrap: true,
      is_esm: false,
      placeholder: Some("JSON_PLACEHOLDER".to_string()),
      is_webworker: true,
      kind: None,
      promise_symbol: None,
      import_attributes: expected_import_attributes,
      media: Some("print".to_string()),
      is_css_import: true,
      chunk_name_magic_comment: Some("json-chunk".to_string()),
    };

    assert_eq!(expected, deserialized);
  }

  #[test]
  fn test_dependency_json_roundtrip_with_all_fields() {
    let mut import_attributes = BTreeMap::new();
    import_attributes.insert("type".to_string(), true);
    import_attributes.insert("with".to_string(), false);

    let dependency = Dependency {
      id: "roundtrip_dep".to_string(),
      bundle_behavior: Default::default(),
      env: Arc::new(Environment::default()),
      specifier: "./roundtrip.ts".to_string(),
      specifier_type: SpecifierType::Url,
      priority: Priority::Parallel,
      loc: None,
      meta: {
        let mut meta = crate::types::JSONObject::new();
        meta.insert("testField".to_string(), serde_json::json!("testValue"));
        meta.insert("numField".to_string(), serde_json::json!(42));
        meta.insert("boolField".to_string(), serde_json::json!(true));
        meta
      },
      package_conditions: Default::default(),
      pipeline: Some("typescript".to_string()),
      range: Some(">=3.0.0".to_string()),
      resolve_from: Some(PathBuf::from("/project/src")),
      source_asset_id: Some("parent_asset_123".to_string()),
      source_path: Some(PathBuf::from("parent.ts")),
      source_asset_type: Some(crate::types::FileType::Ts),
      symbols: None,
      target: None,
      is_entry: true,
      is_optional: false,
      needs_stable_name: true,
      should_wrap: false,
      is_esm: true,
      placeholder: Some("ROUNDTRIP_PLACEHOLDER".to_string()),
      is_webworker: false,
      kind: None,
      promise_symbol: Some("roundtripPromise".to_string()),
      import_attributes,
      media: Some("screen and (max-width: 600px)".to_string()),
      is_css_import: false,
      chunk_name_magic_comment: Some("roundtrip-chunk".to_string()),
    };

    // Serialize to JSON string
    let json_string = serde_json::to_string_pretty(&dependency).expect("Failed to serialize");

    // Deserialize back from JSON string
    let deserialized: Dependency =
      serde_json::from_str(&json_string).expect("Failed to deserialize");

    // Assert entire struct matches
    assert_eq!(dependency, deserialized);
  }

  #[test]
  fn test_dependency_handles_missing_optional_fields() {
    // Test JSON with only required fields
    let minimal_json = r#"{
      "id": "minimal_json_dep",
      "env": {
        "context": "browser",
        "engines": {},
        "includeNodeModules": false,
        "outputFormat": "global",
        "isLibrary": false,
        "shouldOptimize": false,
        "shouldScopeHoist": false,
        "sourceMap": null,
        "sourceType": "module",
        "unstableSingleFileOutput": false
      },
      "specifier": "./minimal.js",
      "specifierType": 0,
      "priority": 0
    }"#;

    let deserialized: Dependency =
      serde_json::from_str(minimal_json).expect("Failed to deserialize minimal JSON");

    // Build expected minimal dependency struct
    let expected = Dependency {
      id: "minimal_json_dep".to_string(),
      bundle_behavior: Default::default(),
      env: Arc::new(Environment {
        context: EnvironmentContext::Browser,
        engines: Default::default(),
        include_node_modules: IncludeNodeModules::Bool(false),
        output_format: OutputFormat::Global,
        is_library: false,
        should_optimize: false,
        should_scope_hoist: false,
        source_map: None,
        source_type: SourceType::Module,
        unstable_single_file_output: false,
        custom_env: None,
        loc: None,
      }),
      specifier: "./minimal.js".to_string(),
      specifier_type: SpecifierType::Esm,
      priority: Priority::Sync,
      ..Default::default()
    };

    assert_eq!(expected, deserialized);
  }
}
