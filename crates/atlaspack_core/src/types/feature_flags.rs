use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureFlagValue {
  String(String),
  Bool(bool),
}

pub type FeatureFlags = HashMap<String, FeatureFlagValue>;
