use std::future::Future;

use serde::{de::DeserializeOwned, Serialize};

use super::InMemoryKVCache;

pub enum KVCache {
  InMemory(InMemoryKVCache),
}

impl KVCache {
  pub async fn set(&self, key: impl AsRef<[u8]>, value: impl Serialize) -> anyhow::Result<()> {
    match self {
      KVCache::InMemory(c) => c.set(key, value).await,
    }
  }

  pub async fn get<D: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> anyhow::Result<Option<D>> {
    match self {
      KVCache::InMemory(c) => c.get(key).await,
    }
  }

  pub async fn get_or_init<
    SD: Serialize + DeserializeOwned,
    Fut: Future<Output = anyhow::Result<SD>>,
  >(
    &self,
    key: impl AsRef<[u8]>,
    func: impl FnOnce() -> Fut,
  ) -> anyhow::Result<SD> {
    match self {
      KVCache::InMemory(c) => c.get_or_init(key, func).await,
    }
  }
}
