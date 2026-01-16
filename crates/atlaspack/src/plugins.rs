use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::plugin::CacheStatus;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;
#[cfg(test)]
use mockall::automock;

pub type PluginsRef = Arc<dyn Plugins + Send + Sync>;

pub mod config_plugins;
pub mod plugin_cache;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Plugins {
  fn named_pipelines(&self) -> Vec<String>;
  fn resolvers(&self) -> Result<Vec<Arc<dyn ResolverPlugin>>, anyhow::Error>;
  async fn transformers(&self, asset: &Asset) -> Result<TransformerPipeline, anyhow::Error>;
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
        match transformer.cache_key().as_ref() {
          CacheStatus::Hash(hash) => {
            hash.hash(&mut cache_key_hasher);
          }
          CacheStatus::Uncachable => {
            cacheable = false;
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
