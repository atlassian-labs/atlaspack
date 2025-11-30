use std::fmt::Debug;

use indexmap::IndexMap;

use crate::utils_hash::hash;

/// Options accepted by the cache implementation. Matches the surface area used
/// by the Babel transformer while defaulting to the same values.
#[derive(Clone, Debug, Default)]
pub struct CacheOptions {
  pub cache: Option<bool>,
  pub max_size: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct CacheConfig {
  cache: bool,
  max_size: usize,
}

impl Default for CacheConfig {
  fn default() -> Self {
    Self {
      cache: true,
      max_size: 500,
    }
  }
}

/// LRU cache used by the Babel plugin to memoize expensive operations such as
/// dependency resolution and AST traversal.
#[derive(Clone, Debug)]
pub struct Cache<T: Clone + Debug> {
  options: CacheConfig,
  entries: IndexMap<String, T>,
  key_strategy: fn(&str, Option<&str>) -> String,
}

impl<T: Clone + Debug> Default for Cache<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Clone + Debug> Cache<T> {
  /// Create a new cache instance with default options.
  pub fn new() -> Self {
    Self {
      options: CacheConfig::default(),
      entries: IndexMap::new(),
      key_strategy: Self::default_key_strategy,
    }
  }

  /// Override the key generation strategy. Used by tests to avoid hashed keys
  /// when asserting insertion order.
  pub fn set_key_strategy(&mut self, strategy: fn(&str, Option<&str>) -> String) {
    self.key_strategy = strategy;
  }

  /// Compute the canonical cache key using the default hashing strategy.
  pub fn get_unique_key(cache_key: &str, namespace: Option<&str>) -> String {
    Self::default_key_strategy(cache_key, namespace)
  }

  fn default_key_strategy(cache_key: &str, namespace: Option<&str>) -> String {
    let combined = match namespace {
      Some(ns) => {
        let mut joined = String::with_capacity(ns.len() + cache_key.len() + 4);
        joined.push_str(ns);
        joined.push_str("----");
        joined.push_str(cache_key);
        joined
      }
      None => cache_key.to_string(),
    };

    hash(&combined)
  }

  /// Initialize the cache with the provided options.
  pub fn initialize(&mut self, options: CacheOptions) {
    let defaults = CacheConfig::default();
    self.options = CacheConfig {
      cache: options.cache.unwrap_or(defaults.cache),
      max_size: options.max_size.unwrap_or(defaults.max_size),
    };
  }

  fn maybe_evict_lru(&mut self) {
    if self.entries.len() >= self.options.max_size {
      self.entries.shift_remove_index(0);
    }
  }

  /// Load a value from the cache, populating it if it does not already exist.
  pub fn load<F>(&mut self, namespace: Option<&str>, cache_key: &str, value: F) -> T
  where
    F: FnOnce() -> T,
  {
    if !self.options.cache {
      return value();
    }

    let unique_key = (self.key_strategy)(cache_key, namespace);

    if let Some(existing) = self.entries.shift_remove(unique_key.as_str()) {
      let result = existing.clone();
      self.entries.insert(unique_key, existing);
      return result;
    }

    self.maybe_evict_lru();

    let computed = value();
    self.entries.insert(unique_key, computed.clone());
    computed
  }

  /// Return the number of cached entries currently stored.
  pub fn get_size(&self) -> usize {
    self.entries.len()
  }

  /// Return the ordered list of cache keys. Primarily used in tests.
  pub fn get_keys(&self) -> Vec<String> {
    self.entries.keys().cloned().collect()
  }

  /// Return the cached values in insertion order.
  pub fn get_values(&self) -> Vec<T> {
    self.entries.values().cloned().collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn caches_values_when_enabled() {
    let mut cache: Cache<i32> = Cache::new();
    cache.initialize(CacheOptions {
      cache: Some(true),
      max_size: None,
    });

    let mut calls = 0;
    let value = cache.load(Some("namespace"), "cacheKey", || {
      calls += 1;
      10
    });

    assert_eq!(value, 10);
    assert_eq!(calls, 1);
    assert_eq!(cache.get_size(), 1);

    let cached = cache.load(Some("namespace"), "cacheKey", || {
      calls += 1;
      20
    });

    assert_eq!(cached, 10);
    assert_eq!(calls, 1);
    assert_eq!(cache.get_size(), 1);

    let expected_key = Cache::<i32>::get_unique_key("cacheKey", Some("namespace"));
    assert_eq!(cache.get_keys(), vec![expected_key]);
  }

  #[test]
  fn bypasses_cache_when_disabled() {
    let mut cache: Cache<i32> = Cache::new();
    cache.initialize(CacheOptions {
      cache: Some(false),
      max_size: None,
    });

    let mut calls = 0;
    let first = cache.load(Some("namespace"), "cacheKey", || {
      calls += 1;
      1
    });
    let second = cache.load(Some("namespace"), "cacheKey", || {
      calls += 1;
      2
    });

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(calls, 2);
    assert_eq!(cache.get_size(), 0);
  }

  #[test]
  fn evicts_least_recently_used_entry() {
    let mut cache: Cache<i32> = Cache::new();
    cache.initialize(CacheOptions {
      cache: Some(true),
      max_size: Some(3),
    });

    let key1 = Cache::<i32>::get_unique_key("cacheKey1", Some("namespace1"));
    let key2 = Cache::<i32>::get_unique_key("cacheKey2", None);
    let key3 = Cache::<i32>::get_unique_key("cacheKey3", Some("namespace3"));
    let key4 = Cache::<i32>::get_unique_key("cacheKey4", Some("namespace4"));

    cache.load(Some("namespace1"), "cacheKey1", || 10);
    cache.load(None, "cacheKey2", || 20);
    cache.load(Some("namespace3"), "cacheKey3", || 30);
    assert_eq!(
      cache.get_keys(),
      vec![key1.clone(), key2.clone(), key3.clone()]
    );

    cache.load(None, "cacheKey2", || 20);
    cache.load(Some("namespace1"), "cacheKey1", || 10);
    assert_eq!(
      cache.get_keys(),
      vec![key3.clone(), key2.clone(), key1.clone()]
    );

    cache.load(Some("namespace4"), "cacheKey4", || 40);
    assert_eq!(
      cache.get_keys(),
      vec![key2.clone(), key1.clone(), key4.clone()]
    );
    assert_eq!(cache.get_values(), vec![20, 10, 40]);
  }
}
