use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureFlagValue {
  String(String),
  Bool(bool),
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeatureFlags(HashMap<String, FeatureFlagValue>);

impl FeatureFlags {
  pub fn get(&self, key: impl AsRef<str>) -> Option<&FeatureFlagValue> {
    self.0.get(key.as_ref())
  }

  /// bool_enabled will return true if a featureFlag is present and
  /// the value is true, otherwise it will return false
  pub fn bool_enabled(&self, key: impl AsRef<str>) -> bool {
    let Some(FeatureFlagValue::Bool(v)) = self.get(key.as_ref()) else {
      return false;
    };
    *v
  }

  /// string_eq will return true if a featureFlag is a string and the string
  /// matches the supplied value, otherwise it will return false
  pub fn string_eq(&self, key: impl AsRef<str>, matches: impl AsRef<str>) -> bool {
    let Some(FeatureFlagValue::String(v)) = self.0.get(key.as_ref()) else {
      return false;
    };
    v == matches.as_ref()
  }

  /// Create a new FeatureFlags instance with a single boolean flag set
  /// This is primarily intended for testing
  pub fn with_bool_flag(key: impl Into<String>, value: bool) -> Self {
    let mut flags = HashMap::new();
    flags.insert(key.into(), FeatureFlagValue::Bool(value));
    FeatureFlags(flags)
  }
}
