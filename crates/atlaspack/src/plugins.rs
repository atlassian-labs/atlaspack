use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::u64;

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
#[cfg(test)]
use mockall::automock;

pub type PluginsRef = Arc<dyn Plugins + Send + Sync>;

pub mod config_plugins;

#[cfg_attr(test, automock)]
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
  fn transformers(
    &self,
    path: &Path,
    pipeline: Option<String>,
  ) -> Result<TransformerPipeline, anyhow::Error>;
  #[allow(unused)]
  fn validators(&self, _path: &Path) -> Result<Vec<Box<dyn ValidatorPlugin>>, anyhow::Error>;
}

#[derive(Default)]
pub struct TransformerPipeline {
  transformers: Vec<Arc<dyn TransformerPlugin>>,
  pipeline_id: u64,
}

#[cfg_attr(test, automock)]
impl TransformerPipeline {
  pub fn new(transformers: Vec<Arc<dyn TransformerPlugin>>) -> Self {
    let mut hasher = atlaspack_core::hash::IdentifierHasher::default();

    for transformer in &transformers {
      transformer.id().hash(&mut hasher);
    }

    Self {
      transformers,
      pipeline_id: hasher.finish(),
    }
  }
  pub fn id(&self) -> u64 {
    self.pipeline_id
  }

  pub fn transformers_mut(&mut self) -> &mut [Arc<dyn TransformerPlugin>] {
    &mut self.transformers
  }
}

impl Debug for TransformerPipeline {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TransformerPipeline")
      .field("transformers", &self.transformers)
      .finish()
  }
}
