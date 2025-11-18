// Simple types for basic alias map representation in configuration
// The actual AliasMap implementation lives in the resolver

use std::collections::BTreeMap;

// Simple alias map type for configuration - just key-value string pairs
pub type AliasMap = BTreeMap<String, String>;
