use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
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

  /// Set a boolean flag only if it doesn't already exist (useful for defaults).
  /// This is also primarily intended for testing
  pub fn with_bool_flag_default(mut self, key: impl Into<String>, default_value: bool) -> Self {
    self
      .0
      .entry(key.into())
      .or_insert(FeatureFlagValue::Bool(default_value));
    self
  }
}

/// Defines a type-safe feature flags struct that can be stored on other structs.
///
/// This macro generates a named struct with methods that only accept the specified flags,
/// providing compile-time safety and making it possible to store the extracted flags.
///
/// # Examples
///
/// ```rust
/// use atlaspack_core::{define_feature_flags, types::{FeatureFlags, FeatureFlagValue}};
///
/// // Define the flags struct type
/// define_feature_flags!(MyTransformerFlags, {
///   experimental_scope_hoisting,
///   swc_minifier,
///   jsx_runtime
/// });
///
/// // Use it in your struct
/// struct MyTransformer {
///   flags: MyTransformerFlags,
/// }
///
/// // Create instance
/// let source_flags = FeatureFlags::default(); // or load from config
/// let flags = MyTransformerFlags::new(&source_flags);
/// let is_enabled = flags.experimental_scope_hoisting(); // ✅ Compiles
/// ```
#[macro_export]
macro_rules! define_feature_flags {
    ($struct_name:ident, { $($flag:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Hash)]
        pub struct $struct_name {
            flags: std::collections::BTreeMap<String, $crate::types::FeatureFlagValue>,
        }

        impl $struct_name {
            pub fn new(source: &$crate::types::FeatureFlags) -> Self {
                let mut extracted = std::collections::BTreeMap::new();
                $(
                    if let Some(value) = source.get(stringify!($flag)) {
                        extracted.insert(stringify!($flag).to_string(), value.clone());
                    }
                )+
                Self { flags: extracted }
            }

            $(
                #[allow(non_snake_case)]
                pub fn $flag(&self) -> bool {
                    if let Some($crate::types::FeatureFlagValue::Bool(v)) = self.flags.get(stringify!($flag)) {
                        *v
                    } else {
                        false
                    }
                }
            )+
        }
    }
}
