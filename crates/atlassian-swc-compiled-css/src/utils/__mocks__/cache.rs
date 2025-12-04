use std::fmt::Debug;

use crate::utils_cache::{Cache as RealCache, CacheOptions};

/// Test-only wrapper that mirrors `packages/babel-plugin/src/utils/__mocks__/cache.ts`.
///
/// The Babel tests replace the hashing strategy with a passthrough function so
/// assertions can reason about human-readable keys. The same behaviour is
/// modelled here by overriding the key generator to join the namespace and cache
/// key with `----`.
pub struct Cache<T: Clone + Debug> {
  inner: RealCache<T>,
}

impl<T: Clone + Debug> Cache<T> {
  pub fn new() -> Self {
    let mut inner = RealCache::new();
    inner.set_key_strategy(|cache_key, namespace| match namespace {
      Some(ns) => {
        let mut joined = String::with_capacity(ns.len() + cache_key.len() + 4);
        joined.push_str(ns);
        joined.push_str("----");
        joined.push_str(cache_key);
        joined
      }
      None => cache_key.to_string(),
    });

    Self { inner }
  }

  pub fn initialize(&mut self, options: CacheOptions) {
    self.inner.initialize(options);
  }

  pub fn load<F>(&mut self, namespace: Option<&str>, cache_key: &str, value: F) -> T
  where
    F: FnOnce() -> T,
  {
    self.inner.load(namespace, cache_key, value)
  }

  pub fn get_size(&self) -> usize {
    self.inner.get_size()
  }

  pub fn get_keys(&self) -> Vec<String> {
    self.inner.get_keys()
  }

  pub fn get_values(&self) -> Vec<T> {
    self.inner.get_values()
  }

  pub fn get_unique_key(cache_key: &str, namespace: Option<&str>) -> String {
    match namespace {
      Some(ns) => format!("{ns}----{cache_key}"),
      None => cache_key.to_string(),
    }
  }
}
