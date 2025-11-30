use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::plugin::BundlerPlugin;
use atlaspack_core::plugin::CompressorPlugin;
use atlaspack_core::plugin::NamerPlugin;
use atlaspack_core::plugin::OptimizerPlugin;
use atlaspack_core::plugin::PackagerPlugin;
use atlaspack_core::plugin::ReporterPlugin;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::RuntimePlugin;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::plugin::ValidatorPlugin;
use atlaspack_core::types::Asset;
#[cfg(test)]
use mockall::automock;
use serde::Serialize;

pub type PluginsRef = Arc<dyn Plugins + Send + Sync>;

pub mod config_plugins;
pub mod plugin_cache;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Plugins {
  #[allow(unused)]
  fn bundler(&self) -> Result<Box<dyn BundlerPlugin>, anyhow::Error>;
  #[allow(unused)]
  fn compressors(&self, path: &Path) -> Result<Vec<Box<dyn CompressorPlugin>>, anyhow::Error>;
  fn named_pipelines(&self) -> Vec<String>;
  #[allow(unused)]
  fn namers(&self) -> Result<Vec<Box<dyn NamerPlugin>>, anyhow::Error>;
  #[allow(unused)]
  fn optimizers(
    &self,
    path: &Path,
    pipeline: Option<String>,
  ) -> Result<Vec<Box<dyn OptimizerPlugin>>, anyhow::Error>;
  #[allow(unused)]
  fn packager(&self, path: &Path) -> Result<Box<dyn PackagerPlugin>, anyhow::Error>;
  fn reporter(&self) -> Arc<dyn ReporterPlugin>;
  fn resolvers(&self) -> Result<Vec<Arc<dyn ResolverPlugin>>, anyhow::Error>;
  #[allow(unused)]
  fn runtimes(&self) -> Result<Vec<Box<dyn RuntimePlugin>>, anyhow::Error>;
  async fn transformers(&self, asset: &Asset) -> Result<TransformerPipeline, anyhow::Error>;
  #[allow(unused)]
  fn validators(&self, _path: &Path) -> Result<Vec<Box<dyn ValidatorPlugin>>, anyhow::Error>;
}

pub struct TransformerPipeline {
  transformers: Vec<Arc<dyn TransformerPlugin>>,
  pipeline_id: u64,
  pub cache_key: Option<u64>,
}

#[cfg_attr(test, automock)]
impl TransformerPipeline {
  pub fn new(transformers: Vec<Arc<dyn TransformerPlugin>>) -> Self {
    let mut id_hasher = atlaspack_core::hash::IdentifierHasher::default();
    let mut cache_key_hasher = atlaspack_core::hash::IdentifierHasher::default();

    let mut cacheable = true;

    for transformer in &transformers {
      transformer.id().hash(&mut id_hasher);

      if cacheable {
        match transformer.cache_key() {
          atlaspack_core::plugin::CacheStatus::Hash(hash) => {
            hash.hash(&mut cache_key_hasher);
          }
          atlaspack_core::plugin::CacheStatus::Uncachable => {
            cacheable = false;
          }
          atlaspack_core::plugin::CacheStatus::BuiltIn => {
            // Built-in transformers are considered cacheable, but do not contribute to the cache key
          }
        }
      }
    }

    let cache_key = if cacheable {
      Some(cache_key_hasher.finish())
    } else {
      None
    };

    Self {
      transformers,
      pipeline_id: id_hasher.finish(),
      cache_key,
    }
  }

  pub fn id(&self) -> u64 {
    self.pipeline_id
  }

  pub fn transformers(&self) -> &[Arc<dyn TransformerPlugin>] {
    &self.transformers
  }

  pub fn should_run_new_pipeline(&self, other: &TransformerPipeline) -> bool {
    if self.pipeline_id == other.pipeline_id {
      // If the pipeline IDs are the same, no need to run
      return false;
    }

    if self.transformers.len() < other.transformers.len() {
      // If the new pipeline has more transformers, we need to run it
      return true;
    }

    for transformer in &other.transformers {
      if !self.transformers.iter().any(|t| t.id() == transformer.id()) {
        // If any transformer is not in the current pipeline, we need to run it
        return true;
      }
    }

    false
  }
}

impl Debug for TransformerPipeline {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TransformerPipeline")
      .field("transformers", &self.transformers)
      .finish()
  }
}
