use std::{collections::HashMap, future::Future, sync::Arc};

use bincode;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemoryKVCache {
  store: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl InMemoryKVCache {
  pub async fn set(&self, key: impl AsRef<[u8]>, value: impl Serialize) -> anyhow::Result<()> {
    let bytes = bincode::serialize(&value)?;
    self
      .store
      .write()
      .await
      .insert(key.as_ref().to_vec(), bytes);
    Ok(())
  }

  pub async fn get<D: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> anyhow::Result<Option<D>> {
    Ok(match self.store.read().await.get(key.as_ref()) {
      Some(bytes) => Some(bincode::deserialize(bytes)?),
      None => None,
    })
  }

  pub async fn get_or_init<
    SD: Serialize + DeserializeOwned,
    Fut: Future<Output = anyhow::Result<SD>>,
  >(
    &self,
    key: impl AsRef<[u8]>,
    func: impl FnOnce() -> Fut,
  ) -> anyhow::Result<SD> {
    if let Some(stored) = self.get::<SD>(&key).await? {
      return Ok(stored);
    }

    let value = func().await?;
    self.set(key, &value).await?;
    Ok(value)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[tokio::test]
  async fn should_set_value() {
    let cache = InMemoryKVCache::default();

    cache.set("hello", "world").await.expect("Should set value");

    let value = cache
      .get::<String>("hello")
      .await
      .expect("Should get value")
      .expect("Should get value");

    assert_eq!(value, "world");
  }

  #[tokio::test]
  async fn should_set_value_func() {
    let cache = InMemoryKVCache::default();

    let value = cache
      .get_or_init("hello", || async { Ok("world".to_string()) })
      .await
      .expect("Should get value");

    assert_eq!(value, "world");

    let value = cache
      .get::<String>("hello")
      .await
      .expect("Should get value")
      .expect("Should get value");

    assert_eq!(value, "world");
  }
}
