use std::{collections::HashMap, sync::Arc};

use atlaspack_core::plugin::{ResolverPlugin, TransformerPlugin};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::future::Future;
use tokio::sync::OnceCell as AsyncOnceCell;

/// A thread safe storage mechanism for plugin instances
#[derive(Default)]
pub struct PluginCache {
  resolvers_store: OnceCell<Vec<Arc<dyn ResolverPlugin>>>,
  transformers_store: RwLock<HashMap<String, Arc<AsyncOnceCell<Arc<dyn TransformerPlugin>>>>>,
}

impl PluginCache {
  pub fn get_or_init_resolvers<F>(&self, f: F) -> anyhow::Result<Vec<Arc<dyn ResolverPlugin>>>
  where
    F: FnOnce() -> anyhow::Result<Vec<Arc<dyn ResolverPlugin>>>,
  {
    self.resolvers_store.get_or_try_init(f).cloned()
  }

  pub async fn get_or_init_transformer<S, F, Fut>(
    &self,
    name: S,
    f: F,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>>
  where
    S: AsRef<str>,
    F: FnOnce() -> Fut + Send,
    Fut: Future<Output = anyhow::Result<Arc<dyn TransformerPlugin>>>,
  {
    let name_str = name.as_ref();

    // Get or create the OnceCell for this transformer name
    let once_cell = {
      // First try with read lock
      if let Some(cell) = self.transformers_store.read().get(name_str) {
        cell.clone()
      } else {
        // Need write lock to insert new OnceCell
        let mut write_guard = self.transformers_store.write();
        // Double-check in case another thread inserted it
        if let Some(cell) = write_guard.get(name_str) {
          cell.clone()
        } else {
          let new_cell = Arc::new(AsyncOnceCell::new());
          write_guard.insert(name_str.to_string(), new_cell.clone());
          new_cell
        }
      }
    };

    // Now use get_or_try_init on the OnceCell - this ensures the async function is only called once
    let transformer = once_cell.get_or_try_init(|| async { f().await }).await?;
    Ok(transformer.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use async_trait::async_trait;
  use atlaspack_core::plugin::{TransformContext, TransformResult};
  use atlaspack_core::types::Asset;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::time::Duration;
  use tokio::time::sleep;

  #[derive(Debug)]
  struct MockTransformer;

  #[async_trait]
  impl TransformerPlugin for MockTransformer {
    async fn transform(
      &self,
      _context: TransformContext,
      asset: Asset,
    ) -> anyhow::Result<TransformResult> {
      Ok(TransformResult {
        asset,
        ..Default::default()
      })
    }
  }

  #[tokio::test]
  async fn test_transformer_only_initialized_once() {
    let cache = Arc::new(PluginCache::default());
    let call_count = Arc::new(AtomicUsize::new(0));

    // Create multiple concurrent tasks that try to initialize the same transformer
    let mut handles = vec![];

    for _ in 0..10 {
      let cache = cache.clone();
      let call_count = call_count.clone();

      let handle = tokio::spawn(async move {
        cache
          .get_or_init_transformer("test_transformer", || {
            let call_count = call_count.clone();
            async move {
              // Increment counter to track how many times this function is called
              call_count.fetch_add(1, Ordering::SeqCst);

              // Add a small delay to increase chance of race conditions
              sleep(Duration::from_millis(10)).await;

              let transformer: Arc<dyn TransformerPlugin> = Arc::new(MockTransformer);
              Ok(transformer)
            }
          })
          .await
      });

      handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
      handle.await.unwrap().unwrap();
    }

    // Verify the async function was only called once despite 10 concurrent attempts
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
  }
}
