use std::{borrow::Borrow, collections::HashMap, hash::BuildHasher, sync::RwLock};

/// A HashMap that allows sharing of values between threads
/// It is currently implemented using RwLock
pub struct SharedHashMap<
  K: Send + Eq,
  V: Send + Clone,
  H: Send + Default + BuildHasher = xxhash_rust::xxh3::Xxh3Builder,
> {
  inner: RwLock<HashMap<K, V, H>>,
}

impl<K, V, H> Default for SharedHashMap<K, V, H>
where
  K: Send + Eq,
  V: Send + Clone,
  H: Send + Default + BuildHasher,
{
  fn default() -> Self {
    Self {
      inner: RwLock::new(HashMap::with_hasher(H::default())),
    }
  }
}

impl<K, V, H> SharedHashMap<K, V, H>
where
  K: std::hash::Hash + Send + Eq,
  V: Send + Clone,
  H: Send + Default + BuildHasher,
{
  pub fn new() -> Self {
    Self {
      inner: RwLock::new(HashMap::with_hasher(H::default())),
    }
  }

  pub fn get<KR>(&self, key: &KR) -> Option<V>
  where
    KR: ?Sized,
    K: Borrow<KR>,
    KR: Eq + std::hash::Hash,
  {
    let map_cell = self.inner.read().unwrap();
    let map = map_cell;
    map.get(key).cloned()
  }

  pub fn insert(&self, key: K, value: V) {
    let map_cell = self.inner.write().unwrap();
    let mut map = map_cell;
    map.insert(key, value);
  }
}

#[cfg(test)]
mod test {
  use std::sync::Arc;

  use super::*;

  #[test]
  fn test_shared_hash_map() {
    let map = SharedHashMap::<String, String>::new();
    map.insert("key".to_string(), "value".to_string());
    assert_eq!(map.get("key"), Some("value".to_string()));
  }

  #[test]
  fn test_multiple_shared_hash_map() {
    let map = Arc::new(SharedHashMap::<String, String>::new());
    let (close_tx, close_rx) = std::sync::mpsc::channel();
    let (tx, rx) = std::sync::mpsc::channel();

    let thread1 = std::thread::spawn({
      let map = map.clone();
      move || {
        map.insert("key".to_string(), "value".to_string());
        assert_eq!(map.get("key"), Some("value".to_string()));
        tx.send(()).unwrap();
        close_rx.recv().unwrap();
      }
    });

    rx.recv().unwrap(); // wait for value to be written
                        // and check value is visible on other threads
    std::thread::spawn({
      let map = map.clone();
      move || {
        assert_eq!(map.get("key"), Some("value".to_string()));
      }
    })
    .join()
    .unwrap();

    // stop
    close_tx.send(()).unwrap();
    thread1.join().unwrap();
  }
}
