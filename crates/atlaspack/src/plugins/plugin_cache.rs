use std::{collections::HashMap, sync::Arc};

use atlaspack_core::plugin::{ResolverPlugin, TransformerPlugin};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;

/// A thread safe storage mechanism for plugin instances
#[derive(Default)]
pub struct PluginCache {
  resolvers_store: OnceCell<Vec<Arc<dyn ResolverPlugin>>>,
  transformers_store: RwLock<HashMap<String, Arc<dyn TransformerPlugin>>>,
}

impl PluginCache {
  pub fn get_or_init_resolvers<F>(&self, f: F) -> anyhow::Result<Vec<Arc<dyn ResolverPlugin>>>
  where
    F: FnOnce() -> anyhow::Result<Vec<Arc<dyn ResolverPlugin>>>,
  {
    self.resolvers_store.get_or_try_init(f).cloned()
  }

  pub fn get_or_init_transformer<S, F>(
    &self,
    name: S,
    f: F,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>>
  where
    S: AsRef<str>,
    F: FnOnce() -> anyhow::Result<Arc<dyn TransformerPlugin>>,
  {
    if let Some(transformer) = self.transformers_store.read().get(name.as_ref()) {
      return Ok(transformer.clone());
    }

    let transformer = f()?;
    self
      .transformers_store
      .write()
      .insert(name.as_ref().to_string(), transformer.clone());
    Ok(transformer)
  }
}
