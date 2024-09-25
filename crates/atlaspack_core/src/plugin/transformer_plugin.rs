use std::path::PathBuf;
use std::{fmt::Debug, sync::Arc};

use serde::Serialize;

use crate::types::{Asset, Dependency, Environment, SpecifierType};

pub struct ResolveOptions {
  /// A list of custom conditions to use when resolving package.json "exports" and "imports"
  pub package_conditions: Vec<String>,
  /// How the specifier should be interpreted
  pub specifier_type: SpecifierType,
}

/// A function that enables transformers to resolve a dependency specifier
pub type Resolve = dyn Fn(PathBuf, String, ResolveOptions) -> Result<PathBuf, anyhow::Error>;

#[derive(Debug, Serialize, PartialEq)]
pub struct TransformResult {
  pub asset: Asset,
  pub dependencies: Vec<Dependency>,
  /// The transformer signals through this field that its result should be invalidated
  /// if these paths change.
  pub invalidate_on_file_change: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct TransformContext {
  environment: Arc<Environment>,
}

impl TransformContext {
  pub fn new(environment: Arc<Environment>) -> Self {
    Self { environment }
  }

  pub fn env(&self) -> &Arc<Environment> {
    &self.environment
  }
}

/// Compile a single asset, discover dependencies, or convert the asset to a different format
///
/// Many transformers are wrappers around other tools such as compilers and preprocessors, and are
/// designed to integrate with Atlaspack.
pub trait TransformerPlugin: Debug + Send + Sync {
  /// Transform the asset and/or add new assets
  fn transform(
    &mut self,
    context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, anyhow::Error>;
}
