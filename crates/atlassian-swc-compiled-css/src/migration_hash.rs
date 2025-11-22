use std::{
  collections::HashMap,
  fs,
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
  sync::{Arc, LazyLock},
};

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde_json::from_str;
use xxhash_rust::xxh3::Xxh3;

use crate::CompiledCssInJsTransformConfig;

pub type IdentifierHasher = Xxh3;

/// Hash the code consistently so we can uniquely identify it for migration purposes.
pub fn hash_code(code: &str) -> String {
  let mut hasher = IdentifierHasher::default();

  code.hash(&mut hasher);

  format!("{}", hasher.finish())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationMap {
  safe_assets: HashMap<String, String>,
}

static SHARED_ASSETS_MAP_DATA: LazyLock<Mutex<HashMap<String, Arc<MigrationMap>>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_or_load_safe_assets_map_from_json(path: String) -> Result<Arc<MigrationMap>> {
  let path_buf = PathBuf::from(&path);
  let path_str = path_buf.to_string_lossy().to_string();
  let mut cache = SHARED_ASSETS_MAP_DATA.lock();

  if let Some(cached_map) = cache.get(&path_str) {
    return Ok(cached_map.clone());
  }

  // Load token map for the first time
  let assets_map = Arc::new(load_safe_assets_map_from_json(&path_buf)?);

  cache.insert(path_str, assets_map.clone());

  Ok(assets_map)
}

fn load_safe_assets_map_from_json(config_path: &Path) -> Result<MigrationMap> {
  let mut json_path = config_path.to_path_buf();

  // Replace the last component of the path with "compiled-css-migration-map.json"
  json_path.pop();
  json_path.push("compiled-css-migration-map.json");

  let json_content = fs::read_to_string(&json_path)
    .with_context(|| format!("Failed to read {}", json_path.display()))?;

  let data: MigrationMap =
    from_str(&json_content).with_context(|| format!("Failed to parse {}", json_path.display()))?;

  Ok(data)
}

pub fn is_safe(code_hash: &str, config: &CompiledCssInJsTransformConfig) -> Result<bool> {
  if !config.unsafe_use_safe_assets {
    // Return true if we're not configured to use safe assets map
    return Ok(true);
  }

  if config.unsafe_report_safe_assets_for_migration {
    return Err(anyhow::anyhow!(
      "unsafe_report_safe_assets_for_migration cannot be used with unsafe_use_safe_assets"
    ));
  }

  if let Some(config_path) = &config.config_path {
    let safe_map = get_or_load_safe_assets_map_from_json(config_path.into());

    safe_map.map(|safe_map| safe_map.safe_assets.contains_key(code_hash))
  } else {
    Err(anyhow::anyhow!(
      "config_path is required when unsafe_use_safe_assets is enabled"
    ))
  }
}
