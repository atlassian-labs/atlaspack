use crate::hash::IdentifierHasher;
use crate::types::{Asset, AssetWithDependencies, Dependency, SpecifierType, Symbol};
use async_trait::async_trait;
use serde::Serialize;
use std::any::Any;
use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub struct ResolveOptions {
  /// A list of custom conditions to use when resolving package.json "exports" and "imports"
  pub package_conditions: Vec<String>,
  /// How the specifier should be interpreted
  pub specifier_type: SpecifierType,
}

/// A function that enables transformers to resolve a dependency specifier
pub type Resolve = dyn Fn(PathBuf, String, ResolveOptions) -> Result<PathBuf, anyhow::Error>;

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct TransformResult {
  pub asset: Asset,
  pub dependencies: Vec<Dependency>,
  pub discovered_assets: Vec<AssetWithDependencies>,
  /// The transformer signals through this field that its result should be invalidated
  /// if these paths change.
  pub invalidate_on_file_change: Vec<PathBuf>,
  pub cache_bailout: bool,
  /// Symbol information for demand-driven symbol resolution
  pub symbol_info: TransformSymbolInfo,
}

/// Symbol information collected during transformation
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct TransformSymbolInfo {
  /// Symbols that this asset exports
  pub exports: Vec<Symbol>,
  /// Symbol requests made by this asset (imports)
  pub symbol_requests: Vec<SymbolRequest>,
  /// Re-export information for barrel file handling
  pub reexports: Vec<ReexportInfo>,
}

/// A symbol request discovered during transformation
#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct SymbolRequest {
  /// The symbol name being imported
  pub symbol: String,
  /// Index of the dependency that should provide this symbol
  pub dependency_index: usize,
  /// The local name this symbol will have in the importing asset
  pub local_name: String,
  /// How the symbol is imported
  pub import_kind: ImportKind,
  /// Source location for error reporting
  pub source_location: Option<crate::types::SourceLocation>,
}

/// Different ways a symbol can be imported
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum ImportKind {
  /// import { foo } from './bar'
  Named(String),
  /// import * as foo from './bar'  
  Namespace,
  /// import foo from './bar'
  Default,
}

/// Re-export information for barrel file detection
#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct ReexportInfo {
  /// The dependency being re-exported from
  pub dependency_index: usize,
  /// Specific symbols being re-exported, or None for export *
  pub symbols: Option<Vec<String>>,
  /// Whether this is an export * (namespace re-export)
  pub is_namespace: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CacheStatus {
  Hash(u64),
  Uncachable,
}

/// Creates a CacheStatus::Hash by hashing the provided values.
///
/// This macro takes any number of arguments that implement Hash and combines
/// them into a single u64 hash for use as a cache key.
#[macro_export]
macro_rules! cache_key {
    ($($val:expr),+ $(,)?) => {{
        use std::hash::{Hash, Hasher};
        use $crate::hash::IdentifierHasher;
        use $crate::plugin::CacheStatus;
        use $crate::version::atlaspack_rust_version;

        let mut hasher = IdentifierHasher::new();
        $(
            $val.hash(&mut hasher);
        )+

        // Always add the @atlaspack/rust version to the cache key
        atlaspack_rust_version().hash(&mut hasher);

        CacheStatus::Hash(hasher.finish())
    }};
}

/// Compile a single asset, discover dependencies, or convert the asset to a different format
///
/// Many transformers are wrappers around other tools such as compilers and preprocessors, and are
/// designed to integrate with Atlaspack.
///
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait TransformerPlugin: Any + Debug + Send + Sync + CacheKey {
  /// Unique ID for this transformer
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    hasher.finish()
  }

  /// Determine whether the transformer should skip transforming the given asset
  fn should_skip(&self, _asset: &Asset) -> anyhow::Result<bool> {
    Ok(false)
  }

  /// Transform the asset and/or add new assets
  async fn transform(&self, asset: Asset) -> anyhow::Result<TransformResult>;
}

pub trait CacheKey {
  fn cache_key(&self) -> Cow<'_, CacheStatus>;
}

// Automatically implement CacheKey for all types that implement Hash
impl<T: Hash> CacheKey for T {
  fn cache_key(&self) -> Cow<'_, CacheStatus> {
    Cow::Owned(cache_key!(self, atlaspack_rust_version()))
  }
}

// Manual CacheKey implementation for MockTransformerPlugin since mockall doesn't generate Hash
#[cfg(test)]
impl CacheKey for MockTransformerPlugin {
  fn cache_key(&self) -> Cow<'_, CacheStatus> {
    // Use the mocked method from the TransformerPlugin trait
    // Note: This will delegate to the mock expectation set up in tests
    Cow::Owned(CacheStatus::Uncachable)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Hash)]
  struct TestConfig {
    value: String,
  }

  #[test]
  fn test_cache_key_macro_single_value() {
    let key = cache_key!(42u64);

    match key {
      CacheStatus::Hash(hash_value) => {
        // Should produce a deterministic hash
        assert!(hash_value > 0);
      }
      _ => panic!("Expected Hash variant"),
    }
  }

  #[test]
  fn test_cache_key_macro_multiple_values() {
    let config = TestConfig {
      value: "test".to_string(),
    };

    let key = cache_key!(config, atlaspack_rust_version(), "extra_string");

    match key {
      CacheStatus::Hash(hash_value) => {
        assert!(hash_value > 0);

        // Same inputs should produce same hash
        let config2 = TestConfig {
          value: "test".to_string(),
        };
        let key2 = cache_key!(config2, atlaspack_rust_version(), "extra_string");

        if let CacheStatus::Hash(hash_value2) = key2 {
          assert_eq!(
            hash_value, hash_value2,
            "Same inputs should produce same hash"
          );
        } else {
          panic!("Expected Hash variant");
        }
      }
      _ => panic!("Expected Hash variant"),
    }
  }

  #[test]
  fn test_cache_key_macro_trailing_comma() {
    // Test that trailing comma works
    let key = cache_key!(42u64, "test",);

    match key {
      CacheStatus::Hash(_) => {} // Success
      _ => panic!("Expected Hash variant"),
    }
  }
}
