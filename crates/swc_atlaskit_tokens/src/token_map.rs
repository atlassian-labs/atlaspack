use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use serde_json5::from_str;
use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
  sync::{Arc, LazyLock},
};

/// Represents a collection of design tokens organized by category
#[derive(Debug, Clone)]
pub struct TokenMap {
  pub token_names: HashMap<String, String>,
  pub light_values: HashMap<String, String>,
  pub legacy_light_values: HashMap<String, String>,
  pub shape_values: HashMap<String, String>,
  pub spacing_values: HashMap<String, String>,
  pub typography_values: HashMap<String, String>,
}

/// Categories of design tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenCategory {
  TokenNames,
  Light,
  LegacyLight,
  Shape,
  Spacing,
  Typography,
}

impl Default for TokenMap {
  fn default() -> Self {
    Self::new()
  }
}

impl TokenMap {
  /// Creates a new empty TokenMap
  pub fn new() -> Self {
    Self {
      token_names: HashMap::new(),
      light_values: HashMap::new(),
      legacy_light_values: HashMap::new(),
      shape_values: HashMap::new(),
      spacing_values: HashMap::new(),
      typography_values: HashMap::new(),
    }
  }

  /// Gets a value from a specific category
  pub fn get_value(&self, category: TokenCategory, key: &str) -> Option<&String> {
    match category {
      TokenCategory::TokenNames => self.token_names.get(key),
      TokenCategory::Light => self.light_values.get(key),
      TokenCategory::LegacyLight => self.legacy_light_values.get(key),
      TokenCategory::Shape => self.shape_values.get(key),
      TokenCategory::Spacing => self.spacing_values.get(key),
      TokenCategory::Typography => self.typography_values.get(key),
    }
  }

  /// Checks if a key exists in a specific category
  pub fn contains_key(&self, category: TokenCategory, key: &str) -> bool {
    match category {
      TokenCategory::TokenNames => self.token_names.contains_key(key),
      TokenCategory::Light => self.light_values.contains_key(key),
      TokenCategory::LegacyLight => self.legacy_light_values.contains_key(key),
      TokenCategory::Shape => self.shape_values.contains_key(key),
      TokenCategory::Spacing => self.spacing_values.contains_key(key),
      TokenCategory::Typography => self.typography_values.contains_key(key),
    }
  }
}

static SHARED_TOKEN_DATA: LazyLock<Mutex<HashMap<String, Arc<TokenMap>>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn get_or_load_token_map_from_json<P: AsRef<Path>>(
  path: Option<P>,
) -> Result<Option<Arc<TokenMap>>> {
  let path = match path {
    Some(p) => p.as_ref().to_path_buf(),
    None => return Ok(None),
  };

  let path_str = path.to_string_lossy().to_string();
  let mut cache = SHARED_TOKEN_DATA.lock();

  if let Some(cached_map) = cache.get(&path_str) {
    return Ok(Some(cached_map.clone()));
  }

  // Load token map for the first time
  let token_map = Arc::new(load_token_map_from_json(&path)?);
  cache.insert(path_str, token_map.clone());
  Ok(Some(token_map))
}

pub fn load_token_map_from_json(json_path: &PathBuf) -> Result<TokenMap> {
  let json_content = fs::read_to_string(json_path)
    .with_context(|| format!("Failed to read {}", json_path.display()))?;

  let data: Value =
    from_str(&json_content).with_context(|| format!("Failed to parse {}", json_path.display()))?;

  let mut token_map = TokenMap::new();

  let load_map = |map: &mut HashMap<String, String>, key: &str| {
    if let Some(values) = data.get(key).and_then(|v| v.as_object()) {
      for (k, v) in values {
        if let Some(val_str) = v.as_str() {
          map.insert(k.clone(), val_str.to_string());
        }
      }
    }
  };

  load_map(&mut token_map.token_names, "tokenNames");
  load_map(&mut token_map.light_values, "light");
  load_map(&mut token_map.legacy_light_values, "legacyLight");
  load_map(&mut token_map.shape_values, "shape");
  load_map(&mut token_map.spacing_values, "spacing");
  load_map(&mut token_map.typography_values, "typography");

  Ok(token_map)
}

/// Clears the token cache (useful for testing)
pub fn clear_token_cache() {
  SHARED_TOKEN_DATA.lock().clear();
}

/// Gets cache statistics
pub fn get_cache_stats() -> (usize, usize) {
  let cache = SHARED_TOKEN_DATA.lock();
  let total_entries = cache.len();
  let total_tokens: usize = cache
    .values()
    .map(|map| {
      map.token_names.len()
        + map.light_values.len()
        + map.legacy_light_values.len()
        + map.shape_values.len()
        + map.spacing_values.len()
        + map.typography_values.len()
    })
    .sum();
  (total_entries, total_tokens)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::NamedTempFile;

  #[test]
  fn test_token_map_initialization() {
    let token_map = TokenMap::new();

    assert!(token_map.token_names.is_empty());
    assert!(token_map.light_values.is_empty());
    assert!(token_map.legacy_light_values.is_empty());
    assert!(token_map.shape_values.is_empty());
    assert!(token_map.spacing_values.is_empty());
    assert!(token_map.typography_values.is_empty());
  }

  #[test]
  fn test_load_token_map_from_json_valid() {
    let json_content = "{
      \"tokenNames\": {
        \"color.background\": \"color-background\",
        \"color.text\": \"color-text\"
      },
      \"light\": {
        \"color.background\": \"#ffffff\",
        \"color.text\": \"#000000\"
      },
      \"legacyLight\": {
        \"color.background\": \"#f8f9fa\",
        \"color.text\": \"#172b4d\"
      },
      \"shape\": {
        \"border.radius\": \"4px\",
        \"border.width\": \"1px\"
      },
      \"spacing\": {
        \"space.100\": \"4px\",
        \"space.200\": \"8px\"
      },
      \"typography\": {
        \"font.size\": \"14px\",
        \"font.weight\": \"400\"
      }
    }";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    let result = load_token_map_from_json(&temp_file.path().to_path_buf());
    assert!(result.is_ok());

    let token_map = result.unwrap();
    assert_eq!(token_map.token_names.len(), 2);
    assert_eq!(token_map.light_values.len(), 2);
    assert_eq!(token_map.legacy_light_values.len(), 2);
    assert_eq!(token_map.shape_values.len(), 2);
    assert_eq!(token_map.spacing_values.len(), 2);
    assert_eq!(token_map.typography_values.len(), 2);

    // Check specific values
    assert!(token_map.token_names.contains_key("color.background"));
    assert!(token_map.light_values.contains_key("color.background"));
    assert!(
      token_map
        .legacy_light_values
        .contains_key("color.background")
    );
    assert!(token_map.shape_values.contains_key("border.radius"));
    assert!(token_map.spacing_values.contains_key("space.100"));
    assert!(token_map.typography_values.contains_key("font.size"));

    // Test new utility methods
    assert_eq!(
      token_map.get_value(TokenCategory::Light, "color.background"),
      Some(&"#ffffff".to_string())
    );
    assert_eq!(
      token_map.get_value(TokenCategory::TokenNames, "color.background"),
      Some(&"color-background".to_string())
    );
    assert!(
      token_map
        .get_value(TokenCategory::Light, "nonexistent")
        .is_none()
    );

    assert!(token_map.contains_key(TokenCategory::Light, "color.background"));
    assert!(!token_map.contains_key(TokenCategory::Light, "nonexistent"));
  }

  #[test]
  fn test_load_token_map_from_json_partial_data() {
    let json_content = "{
      \"tokenNames\": {
        \"color.background\": \"color-background\"
      },
      \"light\": {
        \"color.background\": \"#ffffff\"
      }
    }";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    let result = load_token_map_from_json(&temp_file.path().to_path_buf());
    assert!(result.is_ok());

    let token_map = result.unwrap();
    assert_eq!(token_map.token_names.len(), 1);
    assert_eq!(token_map.light_values.len(), 1);
    assert_eq!(token_map.legacy_light_values.len(), 0);
    assert_eq!(token_map.shape_values.len(), 0);
    assert_eq!(token_map.spacing_values.len(), 0);
    assert_eq!(token_map.typography_values.len(), 0);
  }

  #[test]
  fn test_load_token_map_from_json_empty() {
    let json_content = "{}";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    let result = load_token_map_from_json(&temp_file.path().to_path_buf());
    assert!(result.is_ok());

    let token_map = result.unwrap();
    assert!(token_map.token_names.is_empty());
    assert!(token_map.light_values.is_empty());
    assert!(token_map.legacy_light_values.is_empty());
    assert!(token_map.shape_values.is_empty());
    assert!(token_map.spacing_values.is_empty());
    assert!(token_map.typography_values.is_empty());
  }

  #[test]
  fn test_load_token_map_from_json_invalid_json() {
    let json_content = "{
      \"tokenNames\": {
        \"color.background\": \"color-background\"
      },
      \"light\": {
        \"color.background\": \"#ffffff\"
      }
      // Missing closing brace
    ";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    let result = load_token_map_from_json(&temp_file.path().to_path_buf());
    assert!(result.is_err());
  }

  #[test]
  fn test_load_token_map_from_json_file_not_found() {
    let non_existent_path = PathBuf::from("/non/existent/path.json");
    let result = load_token_map_from_json(&non_existent_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_get_or_load_token_map_from_json_none_path() {
    let result = get_or_load_token_map_from_json::<&str>(None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  }

  #[test]
  fn test_get_or_load_token_map_from_json_valid_path() {
    let json_content = "{
      \"tokenNames\": {
        \"color.background\": \"color-background\"
      },
      \"light\": {
        \"color.background\": \"#ffffff\"
      }
    }";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    let path = temp_file.path().to_string_lossy().to_string();
    let result = get_or_load_token_map_from_json(Some(&path));

    assert!(result.is_ok());
    let token_map_arc = result.unwrap().unwrap();
    assert_eq!(token_map_arc.token_names.len(), 1);
    assert_eq!(token_map_arc.light_values.len(), 1);
  }

  #[test]
  fn test_get_or_load_token_map_from_json_caching() {
    let json_content = "{
      \"tokenNames\": {
        \"color.background\": \"color-background\"
      },
      \"light\": {
        \"color.background\": \"#ffffff\"
      }
    }";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    // Clear cache first
    clear_token_cache();

    let path = temp_file.path();

    // First call - should load from file
    let result1 = get_or_load_token_map_from_json(Some(path));
    assert!(result1.is_ok());
    let token_map_arc1 = result1.unwrap().unwrap();

    // Second call - should return cached version
    let result2 = get_or_load_token_map_from_json(Some(path));
    assert!(result2.is_ok());
    let token_map_arc2 = result2.unwrap().unwrap();

    // Should be the same Arc (cached)
    assert!(Arc::ptr_eq(&token_map_arc1, &token_map_arc2));

    // Test cache stats
    let (entries, tokens) = get_cache_stats();
    assert_eq!(entries, 1);
    assert_eq!(tokens, 2); // 1 token name + 1 light value
  }

  #[test]
  fn test_get_or_load_token_map_from_json_invalid_path() {
    let invalid_path = "/non/existent/path.json";
    let result = get_or_load_token_map_from_json(Some(invalid_path));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to read"));
  }

  #[test]
  fn test_load_map_with_non_string_values() {
    let json_content = "{
      \"tokenNames\": {
        \"color.background\": \"color-background\",
        \"color.number\": 123,
        \"color.boolean\": true
      },
      \"light\": {
        \"color.background\": \"#ffffff\"
      }
    }";

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), json_content).unwrap();

    let result = load_token_map_from_json(&temp_file.path().to_path_buf());
    assert!(result.is_ok());

    let token_map = result.unwrap();
    // Only string values should be included
    assert_eq!(token_map.token_names.len(), 1);
    assert!(token_map.token_names.contains_key("color.background"));
    assert!(!token_map.token_names.contains_key("color.number"));
    assert!(!token_map.token_names.contains_key("color.boolean"));
  }
}
