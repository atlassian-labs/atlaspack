use std::collections::BTreeMap;

// Simple types for basic alias map representation in configuration
// The actual AliasMap implementation lives in the resolver. Ideally we'd just
// have one implementation but we'd need a big refactor to avoid circular
// dependencies.
pub type AliasMap = BTreeMap<String, String>;
