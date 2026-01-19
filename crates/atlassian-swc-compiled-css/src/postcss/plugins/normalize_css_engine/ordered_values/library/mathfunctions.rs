use once_cell::sync::Lazy;
use std::collections::HashSet;

// Port of packages/postcss-plugin-sources/postcss-ordered-values/src/lib/mathfunctions.js
pub fn set() -> &'static HashSet<&'static str> {
  static S: Lazy<HashSet<&'static str>> =
    Lazy::new(|| ["calc", "clamp", "max", "min"].into_iter().collect());
  &S
}
