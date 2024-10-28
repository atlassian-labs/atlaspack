use std::{
  borrow::Borrow,
  cell::RefCell,
  collections::HashMap,
  hash::BuildHasher,
  ops::{Deref, DerefMut},
};

use thread_local::ThreadLocal;

/// A thread-local hash map.
/// This is to be used within a shared structure but disabling the need for
/// actually sharing data between threads.
pub struct ThreadLocalHashMap<
  K: Send + Eq,
  V: Send + Clone,
  H: Send + Default + BuildHasher = xxhash_rust::xxh3::Xxh3Builder,
> {
  inner: ThreadLocal<RefCell<HashMap<K, V, H>>>,
}

impl<K, V, H> Default for ThreadLocalHashMap<K, V, H>
where
  K: Send + Eq,
  V: Send + Clone,
  H: Send + Default + BuildHasher,
{
  fn default() -> Self {
    Self {
      inner: ThreadLocal::new(),
    }
  }
}

impl<K, V, H> ThreadLocalHashMap<K, V, H>
where
  K: std::hash::Hash + Send + Eq,
  V: Send + Clone,
  H: Send + Default + BuildHasher,
{
  pub fn new() -> Self {
    Self {
      inner: ThreadLocal::new(),
    }
  }

  pub fn get<KR>(&self, key: &KR) -> Option<V>
  where
    KR: ?Sized,
    K: Borrow<KR>,
    KR: Eq + std::hash::Hash,
  {
    let map_cell = self
      .inner
      .get_or(|| RefCell::new(HashMap::with_hasher(H::default())));

    let map = map_cell.borrow();
    map.deref().get(key).cloned()
  }

  pub fn insert(&self, key: K, value: V) {
    let map_cell = self
      .inner
      .get_or(|| RefCell::new(HashMap::with_hasher(H::default())));

    let mut map = map_cell.borrow_mut();
    map.deref_mut().insert(key, value);
  }
}

#[cfg(test)]
mod test {
  use std::sync::Arc;

  use super::*;

  #[test]
  fn test_thread_local_hash_map() {
    let map = ThreadLocalHashMap::<String, String>::new();
    map.insert("key".to_string(), "value".to_string());
    assert_eq!(map.get("key"), Some("value".to_string()));
  }

  #[test]
  fn test_multiple_thread_local_hash_map() {
    let map = Arc::new(ThreadLocalHashMap::<String, String>::new());
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
                        // and check value is not visible on other threads
    std::thread::spawn({
      let map = map.clone();
      move || {
        assert_eq!(map.get("key"), None);
      }
    })
    .join()
    .unwrap();

    // stop
    close_tx.send(()).unwrap();
    thread1.join().unwrap();
  }
}
