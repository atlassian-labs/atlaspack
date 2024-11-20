use crate::config_loader::{ConfigLoader, ConfigLoaderRef};
use crate::hash::IdentifierHasher;
use crate::types::{Asset, AssetWithDependencies, Dependency, Environment, SpecifierType};
use async_trait::async_trait;
use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
use mockall::automock;
use serde::Serialize;
use std::any::Any;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

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
  pub dependencies: Vec<Dependency>,
  pub discovered_assets: Vec<AssetWithDependencies>,
  /// The transformer signals through this field that its result should be invalidated
  /// if these paths change.
  pub invalidate_on_file_change: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct TransformContext {
  config: ConfigLoaderRef,
  environment: Arc<Environment>,
}

impl Default for TransformContext {
  fn default() -> Self {
    Self {
      config: Arc::new(ConfigLoader {
        fs: Arc::new(InMemoryFileSystem::default()),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      environment: Arc::new(Environment::default()),
    }
  }
}

impl TransformContext {
  pub fn new(config: ConfigLoaderRef, environment: Arc<Environment>) -> Self {
    Self {
      config,
      environment,
    }
  }

  /// Enables configuration to be loaded from the current asset
  pub fn config(&self) -> ConfigLoaderRef {
    self.config.clone()
  }

  pub fn env(&self) -> &Arc<Environment> {
    &self.environment
  }
}

/// Compile a single asset, discover dependencies, or convert the asset to a different format
///
/// Many transformers are wrappers around other tools such as compilers and preprocessors, and are
/// designed to integrate with Atlaspack.
///
#[automock]
#[async_trait]
pub trait TransformerPlugin: Any + Debug + Send + Sync {
  /// Unique ID for this transformer
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    hasher.finish()
  }

  /// Transform the asset and/or add new assets
  async fn transform(
    &self,
    context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, anyhow::Error>;
}
