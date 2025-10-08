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
    state.serialize_field("id", &self.id)?;
    state.serialize_field("bundleBehavior", &self.bundle_behavior)?;
    state.serialize_field("env", &self.env)?;
    state.serialize_field("filePath", &self.file_path)?;
    state.serialize_field("type", &self.file_type)?;
    state.serialize_field("pipeline", &self.pipeline)?;
    state.serialize_field("query", &self.query)?;
    state.serialize_field("stats", &self.stats)?;
    state.serialize_field("symbols", &self.symbols)?;
    state.serialize_field("sideEffects", &self.side_effects)?;
    state.serialize_field("isBundleSplittable", &self.is_bundle_splittable)?;
    state.serialize_field("isSource", &self.is_source)?;
    state.serialize_field("hasCJSExports", &self.has_cjs_exports)?;
    state.serialize_field("outputHash", &self.output_hash)?;
    state.serialize_field("configPath", &self.config_path)?;
    state.serialize_field("configKeyPath", &self.config_key_path)?;

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
