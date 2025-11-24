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

#[derive(serde::Deserialize, serde::Serialize)]
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

    match safe_map {
      Ok(safe_map) => Ok(safe_map.safe_assets.contains_key(code_hash)),
      Err(e) => Err(e),
    }
  } else {
    Err(anyhow::anyhow!(
      "config_path is required when unsafe_use_safe_assets is enabled"
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;
  use std::fs;
  use tempfile::{TempDir, tempdir};

  struct MockMap {
    config_path: String,
    #[allow(dead_code)]
    temp_dir: TempDir,
  }

  fn mock_map(entries: Vec<(String, String)>) -> MockMap {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let config_path = temp_dir.path().join("config.json");
    fs::write(&config_path, "{}").expect("failed to create config stub");

    let safe_assets: HashMap<String, String> = entries.into_iter().collect();
    let migration_map = MigrationMap { safe_assets };
    let migration_map_path = temp_dir.path().join("compiled-css-migration-map.json");
    fs::write(
      &migration_map_path,
      serde_json::to_string(&migration_map).expect("failed to serialize migration map"),
    )
    .expect("failed to write migration map");

    MockMap {
      config_path: config_path.to_string_lossy().to_string(),
      temp_dir,
    }
  }

  #[test]
  fn in_safe_assets_map() {
    let code_hash = hash_code("test");
    let mock = mock_map(vec![(code_hash.clone(), "migrated.js".to_string())]);
    let config = CompiledCssInJsTransformConfig {
      unsafe_use_safe_assets: true,
      unsafe_report_safe_assets_for_migration: false,
      config_path: Some(mock.config_path.clone()),
      ..Default::default()
    };

    let is_safe_result = is_safe(&code_hash, &config).expect("expected is_safe to succeed");
    assert!(is_safe_result, "safe asset should return true");
  }

  #[test]
  fn not_in_safe_assets_map() {
    let code_hash = hash_code("test");
    let mock = mock_map(vec![]);

    let config = CompiledCssInJsTransformConfig {
      unsafe_use_safe_assets: true,
      unsafe_report_safe_assets_for_migration: false,
      config_path: Some(mock.config_path.clone()),
      ..Default::default()
    };

    let is_safe_result = is_safe(&code_hash, &config).expect("expected is_safe to succeed");
    assert!(!is_safe_result, "safe asset should return false");
  }

  #[test]
  fn not_configured_to_use_safe_assets() {
    let code_hash = hash_code("test");
    let mock = mock_map(vec![]);
    let config = CompiledCssInJsTransformConfig {
      unsafe_use_safe_assets: false,
      unsafe_report_safe_assets_for_migration: false,
      config_path: Some(mock.config_path.clone()),
      ..Default::default()
    };

    let is_safe_result = is_safe(&code_hash, &config).expect("expected is_safe to succeed");
    assert!(
      is_safe_result,
      "all assets should be considered safe if not configured to use safe assets map"
    );
  }

  #[test]
  fn error_if_map_file_is_missing() {
    let code_hash = hash_code("test");
    let config = CompiledCssInJsTransformConfig {
      unsafe_use_safe_assets: true,
      unsafe_report_safe_assets_for_migration: false,
      config_path: Some("nonexistent.json".to_string()),
      ..Default::default()
    };

    assert!(
      is_safe(&code_hash, &config).is_err(),
      "error should be returned if map file is missing"
    );
  }
}
