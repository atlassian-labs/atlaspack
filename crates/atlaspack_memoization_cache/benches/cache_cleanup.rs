use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

// Copy the structs from the main module for benchmarking
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct CacheEntry {
  timestamp: u64,
  key: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheTracker {
  entries: BTreeSet<CacheEntry>,
  key_to_timestamp: HashMap<String, u64>,
}

impl CacheTracker {
  fn update_entry(&mut self, key: String, timestamp: u64) {
    if let Some(old_timestamp) = self.key_to_timestamp.get(&key) {
      let old_entry = CacheEntry {
        timestamp: *old_timestamp,
        key: key.clone(),
      };
      self.entries.remove(&old_entry);
    }

    // Insert new entry
    let new_entry = CacheEntry {
      timestamp,
      key: key.clone(),
    };
    self.entries.insert(new_entry);
    self.key_to_timestamp.insert(key, timestamp);
  }

  fn remove_oldest_entries(&mut self, count: usize) -> Vec<String> {
    let keys_to_remove: Vec<String> = self
      .entries
      .iter()
      .take(count)
      .map(|entry| entry.key.clone())
      .collect();

    for key in &keys_to_remove {
      self.remove_key(key);
    }

    keys_to_remove
  }

  fn remove_key(&mut self, key: &str) {
    if let Some(timestamp) = self.key_to_timestamp.remove(key) {
      let entry = CacheEntry {
        timestamp,
        key: key.to_string(),
      };
      self.entries.remove(&entry);
    }
  }

  fn len(&self) -> usize {
    self.entries.len()
  }
}

fn create_large_cache_tracker(size: usize) -> CacheTracker {
  let mut tracker = CacheTracker::default();

  for i in 0..size {
    let key = format!("cache_key_{:010}", i);
    let timestamp = 1_000_000 + (i as u64);
    tracker.update_entry(key, timestamp);
  }

  tracker
}

fn benchmark_update_entry(c: &mut Criterion) {
  let mut group = c.benchmark_group("update_entry");

  for size in [100_000, 500_000, 1_000_000, 2_000_000] {
    group.bench_with_input(
      BenchmarkId::new("existing_entries", size),
      &size,
      |b, &size| {
        let mut tracker = create_large_cache_tracker(size);
        let mut counter = 0;

        b.iter(|| {
          let key = format!("cache_key_{:010}", counter % size);
          let timestamp = 2_000_000u64 + (counter as u64);
          tracker.update_entry(black_box(key), black_box(timestamp));
          counter += 1;
        });
      },
    );
  }

  group.finish();
}

fn benchmark_remove_oldest_entries(c: &mut Criterion) {
  let mut group = c.benchmark_group("remove_oldest_entries");

  for size in [100_000, 500_000, 1_000_000, 2_000_000] {
    for remove_count in [1000, 10_000, 50_000] {
      if remove_count >= size {
        continue;
      }

      group.bench_with_input(
        BenchmarkId::new(format!("size_{}_remove_{}", size, remove_count), size),
        &(size, remove_count),
        |b, &(size, remove_count)| {
          b.iter_batched(
            || create_large_cache_tracker(size),
            |mut tracker| black_box(tracker.remove_oldest_entries(remove_count)),
            criterion::BatchSize::LargeInput,
          );
        },
      );
    }
  }

  group.finish();
}

fn benchmark_serialization(c: &mut Criterion) {
  let mut group = c.benchmark_group("serialization");

  for size in [100_000, 500_000, 1_000_000] {
    let tracker = create_large_cache_tracker(size);

    group.bench_with_input(
      BenchmarkId::new("serialize", size),
      &tracker,
      |b, tracker| {
        b.iter(|| black_box(bincode::serialize(tracker).unwrap()));
      },
    );

    let serialized = bincode::serialize(&tracker).unwrap();
    group.bench_with_input(
      BenchmarkId::new("deserialize", size),
      &serialized,
      |b, serialized| {
        b.iter(|| black_box(bincode::deserialize::<CacheTracker>(serialized).unwrap()));
      },
    );
  }

  group.finish();
}

fn benchmark_cache_cleanup_scenarios(c: &mut Criterion) {
  let mut group = c.benchmark_group("cleanup_scenarios");

  // Scenario 1: Regular cleanup (remove 10% when over limit)
  for size in [1_000_000, 2_000_000, 5_000_000] {
    let max_entries = (size as f64 * 0.9) as usize;
    let remove_count = size - max_entries;

    group.bench_with_input(
      BenchmarkId::new("regular_cleanup_10pct", size),
      &(size, remove_count),
      |b, &(size, remove_count)| {
        b.iter_batched(
          || create_large_cache_tracker(size),
          |mut tracker| {
            if tracker.len() > max_entries {
              black_box(tracker.remove_oldest_entries(remove_count))
            } else {
              Vec::new()
            }
          },
          criterion::BatchSize::LargeInput,
        );
      },
    );
  }

  // Scenario 2: Aggressive cleanup (remove 50% when over limit)
  for size in [1_000_000, 2_000_000] {
    let remove_count = size / 2;

    group.bench_with_input(
      BenchmarkId::new("aggressive_cleanup_50pct", size),
      &(size, remove_count),
      |b, &(size, remove_count)| {
        b.iter_batched(
          || create_large_cache_tracker(size),
          |mut tracker| black_box(tracker.remove_oldest_entries(remove_count)),
          criterion::BatchSize::LargeInput,
        );
      },
    );
  }

  group.finish();
}

fn benchmark_mixed_operations(c: &mut Criterion) {
  let mut group = c.benchmark_group("mixed_operations");

  // Simulate a realistic workload: mostly updates with occasional cleanup
  for size in [500_000, 1_000_000] {
    group.bench_with_input(
      BenchmarkId::new("realistic_workload", size),
      &size,
      |b, &size| {
        b.iter_batched(
          || create_large_cache_tracker(size),
          |mut tracker| {
            // Simulate 100 cache operations
            for i in 0..100 {
              let key = format!("new_key_{}", i);
              let timestamp = 3_000_000 + i;
              tracker.update_entry(key, timestamp);
            }

            // Every 50 operations, do cleanup
            if tracker.len() > size + 50 {
              tracker.remove_oldest_entries(100);
            }

            black_box(tracker)
          },
          criterion::BatchSize::LargeInput,
        );
      },
    );
  }

  group.finish();
}

fn benchmark_worst_case_scenarios(c: &mut Criterion) {
  let mut group = c.benchmark_group("worst_case");

  // Worst case: Remove almost all entries
  for size in [500_000, 1_000_000] {
    let remove_count = (size as f64 * 0.95) as usize; // Remove 95%

    group.bench_with_input(
      BenchmarkId::new("remove_95_percent", size),
      &(size, remove_count),
      |b, &(size, remove_count)| {
        b.iter_batched(
          || create_large_cache_tracker(size),
          |mut tracker| black_box(tracker.remove_oldest_entries(remove_count)),
          criterion::BatchSize::LargeInput,
        );
      },
    );
  }

  // Memory usage test: Track memory during operations
  group.bench_function("memory_usage_1m_entries", |b| {
    b.iter_batched(
      || create_large_cache_tracker(1_000_000),
      |tracker| {
        // Just measure the memory footprint
        black_box(tracker.len())
      },
      criterion::BatchSize::LargeInput,
    );
  });

  group.finish();
}

criterion_group!(
  benches,
  benchmark_update_entry,
  benchmark_remove_oldest_entries,
  benchmark_serialization,
  benchmark_cache_cleanup_scenarios,
  benchmark_mixed_operations,
  benchmark_worst_case_scenarios,
);
criterion_main!(benches);
