//! Dense bitset with lazy cardinality tracking.
//!
//! This module provides [`DenseBitset`], a simple dense bitset implementation
//! optimized for bulk bitwise operations (AND, OR) where cardinality (popcount)
//! is needed infrequently.
//!
//! # Design rationale
//!
//! In the bundler's availability propagation (Phase 6), we perform ~797k
//! intersection operations but only need cardinality ~7k times. Standard
//! bitmap libraries (e.g., `roaring`) eagerly maintain cardinality on every
//! mutation, which adds significant overhead in intersection-heavy workloads.
//!
//! `DenseBitset` solves this by:
//! - Storing bits in a flat `Box<[u64]>` (no container management overhead)
//! - Tracking cardinality lazily via a dirty flag (`Cell<Option<u64>>`)
//! - Only recomputing popcount when `len()` is explicitly called
//!
//! # Scaling
//!
//! The bitset grows dynamically to accommodate any universe size. It has been
//! tested with 60k elements (Confluence) and is designed to scale to 120k+
//! elements (Jira) without algorithmic changes. Memory usage is proportional
//! to the highest inserted value: `ceil(max_value / 64) * 8` bytes.

use std::cell::Cell;
use std::fmt;

/// A dense bitset with lazy cardinality tracking.
///
/// Optimized for workloads with many bulk AND/OR operations and infrequent
/// cardinality queries. Uses a flat `Box<[u64]>` for storage and defers
/// popcount computation until `len()` is called.
///
/// # Examples
///
/// ```ignore
/// let mut a = DenseBitset::with_capacity(1000);
/// a.insert(42);
/// a.insert(100);
/// assert_eq!(a.len(), 2);
///
/// let mut b = DenseBitset::with_capacity(1000);
/// b.insert(42);
/// b.insert(200);
///
/// a &= &b; // intersection
/// assert_eq!(a.len(), 1);
/// assert!(a.contains(42));
/// assert!(!a.contains(100));
/// ```
#[derive(Clone)]
pub struct DenseBitset {
  /// Bit storage. Word `i` holds bits `i*64` through `i*64 + 63`.
  /// Grows dynamically on `insert` if the value exceeds current capacity.
  pub(crate) words: Box<[u64]>,
  /// Cached cardinality with interior mutability.
  /// `Some(n)` = cardinality is known to be `n`.
  /// `None` = dirty, needs recomputation on next `len()` call.
  /// Uses `Cell` so `len()` can cache the result on `&self`.
  cached_len: Cell<Option<u64>>,
}

impl DenseBitset {
  /// Creates a new empty bitset with no allocated storage.
  ///
  /// The bitset will grow on the first `insert` call. Use [`with_capacity`](Self::with_capacity)
  /// if you know the approximate universe size upfront to avoid repeated reallocations.
  pub fn new() -> Self {
    Self {
      words: vec![0u64; 0].into_boxed_slice(),
      cached_len: Cell::new(Some(0)),
    }
  }

  /// Creates a new empty bitset pre-allocated for values in `0..universe_size`.
  ///
  /// Allocates `ceil(universe_size / 64)` words (8 bytes each). For a 60k-element
  /// universe this is ~7.5KB; for 120k elements, ~15KB.
  pub fn with_capacity(universe_size: usize) -> Self {
    let num_words = (universe_size + 63) / 64;
    Self {
      words: vec![0u64; num_words].into_boxed_slice(),
      cached_len: Cell::new(Some(0)),
    }
  }

  /// Inserts a value into the bitset.
  ///
  /// If the value exceeds the current capacity, the storage is grown to accommodate it.
  /// The cached cardinality is updated precisely (incremented by 1 if the bit was not
  /// already set), avoiding a full recount.
  pub fn insert(&mut self, value: u32) {
    let word_idx = (value / 64) as usize;
    if word_idx >= self.words.len() {
      let new_len = word_idx + 1;
      let mut new_words = vec![0u64; new_len];
      new_words[..self.words.len()].copy_from_slice(&self.words);
      self.words = new_words.into_boxed_slice();
    }
    let bit_idx = value % 64;
    let mask = 1u64 << bit_idx;
    if self.words[word_idx] & mask == 0 {
      self.words[word_idx] |= mask;
      if let Some(count) = self.cached_len.get() {
        self.cached_len.set(Some(count + 1));
      }
    }
  }

  /// Returns `true` if the bitset contains the given value.
  ///
  /// Returns `false` for values beyond the current capacity without panicking.
  pub fn contains(&self, value: u32) -> bool {
    let word_idx = (value / 64) as usize;
    if word_idx >= self.words.len() {
      return false;
    }
    let bit_idx = value % 64;
    self.words[word_idx] & (1u64 << bit_idx) != 0
  }

  /// Returns the number of set bits (cardinality).
  ///
  /// Uses lazy evaluation: if the cardinality was invalidated by a bulk operation
  /// (`&=` or `|=`), it is recomputed via popcount and cached. Subsequent calls
  /// without intervening mutations return the cached value in O(1).
  pub fn len(&self) -> u64 {
    if let Some(count) = self.cached_len.get() {
      count
    } else {
      let count = self.words.iter().map(|w| w.count_ones() as u64).sum();
      self.cached_len.set(Some(count));
      count
    }
  }

  /// Returns `true` if no bits are set.
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Clears all bits, resetting cardinality to zero.
  ///
  /// Does not deallocate storage. Use `DenseBitset::new()` if you want to release memory.
  pub fn clear(&mut self) {
    for w in self.words.iter_mut() {
      *w = 0;
    }
    self.cached_len.set(Some(0));
  }

  /// In-place intersection (`self &= other`).
  ///
  /// Words beyond `other`'s length are zeroed (intersecting with implicit zeros).
  /// Marks cardinality as dirty — it will be recomputed on the next `len()` call.
  ///
  /// This is the hot-path operation in Phase 6 availability propagation,
  /// called ~797k times on the real Confluence app.
  pub fn and_assign(&mut self, other: &Self) {
    let min_len = self.words.len().min(other.words.len());
    for i in 0..min_len {
      self.words[i] &= other.words[i];
    }
    for i in min_len..self.words.len() {
      self.words[i] = 0;
    }
    self.cached_len.set(None);
  }

  /// In-place union (`self |= other`).
  ///
  /// If `other` is wider than `self`, `self` is grown to match.
  /// Marks cardinality as dirty.
  pub fn or_assign(&mut self, other: &Self) {
    if other.words.len() > self.words.len() {
      let mut new_words = vec![0u64; other.words.len()];
      new_words[..self.words.len()].copy_from_slice(&self.words);
      self.words = new_words.into_boxed_slice();
    }
    for i in 0..other.words.len() {
      self.words[i] |= other.words[i];
    }
    self.cached_len.set(None);
  }

  /// Converts this bitset to a [`roaring::RoaringBitmap`].
  ///
  /// Used at the Phase 6 boundary to convert internal `DenseBitset` availability
  /// back to `RoaringBitmap` for storage on `IdealBundle.ancestor_assets`.
  /// Runs once per bundle root (~7,225 times), not in any hot loop.
  pub fn to_roaring(&self) -> roaring::RoaringBitmap {
    let mut rb = roaring::RoaringBitmap::new();
    for (word_idx, &word) in self.words.iter().enumerate() {
      let mut w = word;
      while w != 0 {
        let bit = w.trailing_zeros();
        rb.insert((word_idx as u32) * 64 + bit);
        w &= w - 1;
      }
    }
    rb
  }
}

impl Default for DenseBitset {
  fn default() -> Self {
    Self::new()
  }
}

impl std::ops::BitAndAssign<&DenseBitset> for DenseBitset {
  fn bitand_assign(&mut self, rhs: &DenseBitset) {
    self.and_assign(rhs);
  }
}

impl std::ops::BitOrAssign<&DenseBitset> for DenseBitset {
  fn bitor_assign(&mut self, rhs: &DenseBitset) {
    self.or_assign(rhs);
  }
}

impl fmt::Debug for DenseBitset {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DenseBitset")
      .field("len", &self.len())
      .field("words", &self.words.len())
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_bitset() {
    let bs = DenseBitset::new();
    assert_eq!(bs.len(), 0);
    assert!(bs.is_empty());
    assert!(!bs.contains(0));
    assert!(!bs.contains(1000));
  }

  #[test]
  fn test_with_capacity() {
    let bs = DenseBitset::with_capacity(1000);
    assert_eq!(bs.len(), 0);
    assert!(bs.is_empty());
    // Should have ceil(1000/64) = 16 words
    assert_eq!(bs.words.len(), 16);
  }

  #[test]
  fn test_insert_and_contains() {
    let mut bs = DenseBitset::with_capacity(200);
    bs.insert(0);
    bs.insert(42);
    bs.insert(63);
    bs.insert(64);
    bs.insert(199);

    assert!(bs.contains(0));
    assert!(bs.contains(42));
    assert!(bs.contains(63));
    assert!(bs.contains(64));
    assert!(bs.contains(199));
    assert!(!bs.contains(1));
    assert!(!bs.contains(100));
    assert!(!bs.contains(200));
    assert_eq!(bs.len(), 5);
  }

  #[test]
  fn test_insert_duplicate_does_not_change_len() {
    let mut bs = DenseBitset::with_capacity(100);
    bs.insert(42);
    assert_eq!(bs.len(), 1);
    bs.insert(42);
    assert_eq!(bs.len(), 1);
  }

  #[test]
  fn test_insert_grows_dynamically() {
    let mut bs = DenseBitset::new();
    assert_eq!(bs.words.len(), 0);
    bs.insert(1000);
    assert!(bs.contains(1000));
    assert_eq!(bs.len(), 1);
    // Should have grown to at least word_idx=15 (1000/64)
    assert!(bs.words.len() >= 16);
  }

  #[test]
  fn test_insert_beyond_65536() {
    // Jira will have 120k+ assets
    let mut bs = DenseBitset::new();
    bs.insert(120_000);
    bs.insert(0);
    bs.insert(65_535);
    bs.insert(65_536);
    assert!(bs.contains(120_000));
    assert!(bs.contains(0));
    assert!(bs.contains(65_535));
    assert!(bs.contains(65_536));
    assert!(!bs.contains(60_000));
    assert_eq!(bs.len(), 4);
  }

  #[test]
  fn test_intersection_same_size() {
    let mut a = DenseBitset::with_capacity(200);
    a.insert(1);
    a.insert(2);
    a.insert(3);

    let mut b = DenseBitset::with_capacity(200);
    b.insert(2);
    b.insert(3);
    b.insert(4);

    a &= &b;
    assert!(!a.contains(1));
    assert!(a.contains(2));
    assert!(a.contains(3));
    assert!(!a.contains(4));
    assert_eq!(a.len(), 2);
  }

  #[test]
  fn test_intersection_different_sizes() {
    // a is wider than b — words beyond b's length should be zeroed
    let mut a = DenseBitset::with_capacity(500);
    a.insert(10);
    a.insert(400);

    let mut b = DenseBitset::with_capacity(100);
    b.insert(10);

    a &= &b;
    assert!(a.contains(10));
    assert!(!a.contains(400)); // zeroed because b doesn't extend that far
    assert_eq!(a.len(), 1);
  }

  #[test]
  fn test_intersection_self_shorter_than_other() {
    // b is wider than a — extra bits in b are irrelevant
    let mut a = DenseBitset::with_capacity(100);
    a.insert(10);

    let mut b = DenseBitset::with_capacity(500);
    b.insert(10);
    b.insert(400);

    a &= &b;
    assert!(a.contains(10));
    assert!(!a.contains(400)); // a doesn't have storage for 400
    assert_eq!(a.len(), 1);
  }

  #[test]
  fn test_union_same_size() {
    let mut a = DenseBitset::with_capacity(200);
    a.insert(1);
    a.insert(2);

    let mut b = DenseBitset::with_capacity(200);
    b.insert(2);
    b.insert(3);

    a |= &b;
    assert!(a.contains(1));
    assert!(a.contains(2));
    assert!(a.contains(3));
    assert_eq!(a.len(), 3);
  }

  #[test]
  fn test_union_grows_self() {
    // b is wider than a — a should grow
    let mut a = DenseBitset::with_capacity(100);
    a.insert(10);

    let mut b = DenseBitset::with_capacity(500);
    b.insert(400);

    a |= &b;
    assert!(a.contains(10));
    assert!(a.contains(400));
    assert_eq!(a.len(), 2);
  }

  #[test]
  fn test_clear() {
    let mut bs = DenseBitset::with_capacity(200);
    bs.insert(1);
    bs.insert(100);
    assert_eq!(bs.len(), 2);

    bs.clear();
    assert_eq!(bs.len(), 0);
    assert!(bs.is_empty());
    assert!(!bs.contains(1));
    assert!(!bs.contains(100));
    // Storage is preserved
    assert_eq!(bs.words.len(), 4); // ceil(200/64) = 4
  }

  #[test]
  fn test_lazy_cardinality_after_intersection() {
    let mut a = DenseBitset::with_capacity(200);
    for i in 0..100 {
      a.insert(i);
    }
    assert_eq!(a.len(), 100);

    let mut b = DenseBitset::with_capacity(200);
    for i in 50..150 {
      b.insert(i);
    }

    // After intersection, cached_len should be dirty
    a &= &b;
    // But len() should recompute correctly
    assert_eq!(a.len(), 50); // 50..100
    // Second call should use cache
    assert_eq!(a.len(), 50);
  }

  #[test]
  fn test_lazy_cardinality_caches_on_len() {
    let mut a = DenseBitset::with_capacity(200);
    for i in 0..10 {
      a.insert(i);
    }
    let mut b = DenseBitset::with_capacity(200);
    for i in 0..5 {
      b.insert(i);
    }
    a &= &b;
    // First len() recomputes and caches
    assert_eq!(a.len(), 5);
    // Verify cache is set (via Cell)
    assert_eq!(a.cached_len.get(), Some(5));
  }

  #[test]
  fn test_clone_preserves_state() {
    let mut a = DenseBitset::with_capacity(200);
    a.insert(42);
    a.insert(100);

    let b = a.clone();
    assert_eq!(b.len(), 2);
    assert!(b.contains(42));
    assert!(b.contains(100));
  }

  #[test]
  fn test_to_roaring_roundtrip() {
    let mut bs = DenseBitset::with_capacity(1000);
    let values: Vec<u32> = vec![0, 1, 63, 64, 65, 127, 128, 500, 999];
    for &v in &values {
      bs.insert(v);
    }

    let rb = bs.to_roaring();
    assert_eq!(rb.len(), values.len() as u64);
    for &v in &values {
      assert!(rb.contains(v), "RoaringBitmap should contain {v}");
    }
  }

  #[test]
  fn test_to_roaring_large_universe() {
    // Test with values beyond 65,536 (Jira scale)
    let mut bs = DenseBitset::new();
    bs.insert(0);
    bs.insert(65_535);
    bs.insert(65_536);
    bs.insert(120_000);

    let rb = bs.to_roaring();
    assert_eq!(rb.len(), 4);
    assert!(rb.contains(0));
    assert!(rb.contains(65_535));
    assert!(rb.contains(65_536));
    assert!(rb.contains(120_000));
  }

  #[test]
  fn test_intersection_empties_result() {
    let mut a = DenseBitset::with_capacity(200);
    a.insert(1);
    a.insert(2);

    let mut b = DenseBitset::with_capacity(200);
    b.insert(3);
    b.insert(4);

    a &= &b;
    assert_eq!(a.len(), 0);
    assert!(a.is_empty());
  }

  #[test]
  fn test_word_boundary_values() {
    // Test values at word boundaries (multiples of 64)
    let mut bs = DenseBitset::with_capacity(300);
    for i in (0..300).step_by(64) {
      bs.insert(i);
    }
    // 0, 64, 128, 192, 256 = 5 values
    assert_eq!(bs.len(), 5);
    for i in (0..300).step_by(64) {
      assert!(bs.contains(i), "should contain {i}");
    }
    // Values just before boundaries should NOT be set
    assert!(!bs.contains(63));
    assert!(!bs.contains(127));
  }

  #[test]
  fn test_multiple_operations_sequence() {
    // Simulate a simplified availability propagation:
    // parent1 has {1,2,3,4}, parent2 has {2,3,4,5}
    // child = parent1 & parent2 = {2,3,4}
    // then child |= group_union {10,11}
    // result = {2,3,4,10,11}

    let mut parent1 = DenseBitset::with_capacity(100);
    for v in [1, 2, 3, 4] {
      parent1.insert(v);
    }

    let mut parent2 = DenseBitset::with_capacity(100);
    for v in [2, 3, 4, 5] {
      parent2.insert(v);
    }

    // child starts as clone of first parent
    let mut child = parent1.clone();
    // intersect with second parent
    child &= &parent2;
    assert_eq!(child.len(), 3);

    // union with group
    let mut group = DenseBitset::with_capacity(100);
    group.insert(10);
    group.insert(11);
    child |= &group;

    assert_eq!(child.len(), 5);
    assert!(!child.contains(1));
    assert!(child.contains(2));
    assert!(child.contains(3));
    assert!(child.contains(4));
    assert!(!child.contains(5));
    assert!(child.contains(10));
    assert!(child.contains(11));
  }
}
