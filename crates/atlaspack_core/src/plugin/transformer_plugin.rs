use crate::hash::IdentifierHasher;
use crate::types::{Asset, Dependency, SpecifierType};
use mockall::automock;
use serde::Serialize;
use std::any::Any;
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

#[derive(Debug, Serialize, PartialEq, Default)]
pub struct TransformResult {
  pub asset: Asset,
  pub discovered_assets: Vec<Asset>,
  pub dependencies: Vec<Dependency>,
  /// The transformer signals through this field that its result should be invalidated
  /// if these paths change.
  pub invalidate_on_file_change: Vec<PathBuf>,
}

/// Compile a single asset, discover dependencies, or convert the asset to a different format
///
/// Many transformers are wrappers around other tools such as compilers and preprocessors, and are
/// designed to integrate with Atlaspack.
///
#[automock]
pub trait TransformerPlugin: Any + Debug + Send + Sync {
  /// Unique ID for this transformer
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    hasher.finish()
  }
  /// Transform the asset and/or add new assets
  fn transform(&mut self, input: Asset) -> Result<TransformResult, anyhow::Error>;
}
