use std::any::Any;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::hash::IdentifierHasher;
use crate::types::Dependency;
use crate::types::Invalidation;
use crate::types::JSONObject;
use crate::types::Priority;

// TODO Diagnostics and invalidations

pub struct ResolveContext {
  pub dependency: Arc<Dependency>,
  pub pipeline: Option<String>,
  pub specifier: String,
}

#[derive(Clone, Debug, Hash, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedResolution {
  /// Whether this dependency can be deferred by Atlaspack itself
  pub can_defer: bool,

  /// The code of the resolved asset
  ///
  /// If provided, this is used rather than reading the file from disk.
  ///
  pub code: Option<String>,

  /// An absolute path to the resolved file
  pub file_path: PathBuf,

  /// Is spread (shallowly merged) onto the request's dependency.meta
  pub meta: Option<JSONObject>,

  /// An optional named pipeline to compile the resolved file
  pub pipeline: Option<String>,

  /// Overrides the priority set on the dependency
  pub priority: Option<Priority>,

  /// Query parameters to be used by transformers when compiling the resolved file
  pub query: Option<String>,

  /// Corresponds to the asset side effects
  pub side_effects: bool,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Resolution {
  /// Indicates the dependency was not resolved
  Unresolved,

  /// Whether the resolved file should be excluded from the build
  Excluded,

  Resolved(ResolvedResolution),
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
  pub invalidations: Vec<Invalidation>,
  pub resolution: Resolution,
}

/// Converts a dependency specifier into a file path that will be processed by transformers
///
/// Resolvers run in a pipeline until one of them return a result.
///
#[async_trait]
pub trait ResolverPlugin: Any + Debug + Send + Sync {
  /// Unique ID for this resolver
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    hasher.finish()
  }
  /// Determines what the dependency specifier resolves to
  async fn resolve(&self, ctx: ResolveContext) -> Result<Resolved, anyhow::Error>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug, Hash)]
  struct TestResolverPlugin {}

  #[async_trait]
  impl ResolverPlugin for TestResolverPlugin {
    async fn resolve(&self, _ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
      todo!()
    }
  }

  #[test]
  fn can_be_defined_in_dyn_vec() {
    let mut resolvers = Vec::<Box<dyn ResolverPlugin>>::new();

    resolvers.push(Box::new(TestResolverPlugin {}));

    assert_eq!(resolvers.len(), 1);
  }
}
