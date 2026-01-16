use memory_stats::memory_stats;
use napi_derive::napi;
#[allow(clippy::disallowed_types)]
use std::sync::{
  Mutex,
  atomic::{AtomicU64, Ordering},
};

static BASELINE_PHYSICAL: AtomicU64 = AtomicU64::new(0);
static BASELINE_VIRTUAL: AtomicU64 = AtomicU64::new(0);
static SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);

// Store all samples for detailed statistics
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
static MEMORY_SAMPLES: Mutex<Vec<MemorySample>> = Mutex::new(Vec::new());

#[derive(Clone, Copy, Debug)]
struct MemorySample {
  physical_mem: u64,
  virtual_mem: u64,
}

#[napi(object)]
pub struct DetailedMemoryStats {
  pub min: f64,
  pub max: f64,
  pub mean: f64,
  pub median: f64,
  pub p95: f64,
  pub p99: f64,
  pub standard_deviation: f64,
  pub range: f64,
}

#[napi(object)]
pub struct NativeMemoryStats {
  pub physical_mem: DetailedMemoryStats,
  pub virtual_mem: DetailedMemoryStats,
  pub sample_count: f64,
}

fn calculate_detailed_stats(values: &[u64]) -> DetailedMemoryStats {
  if values.is_empty() {
    return DetailedMemoryStats {
      min: 0.0,
      max: 0.0,
      mean: 0.0,
      median: 0.0,
      p95: 0.0,
      p99: 0.0,
      standard_deviation: 0.0,
      range: 0.0,
    };
  }

  let mut sorted = values.to_vec();
  sorted.sort_unstable();

  let min = sorted[0] as f64;
  let max = sorted[sorted.len() - 1] as f64;
  let mean = values.iter().map(|&x| x as f64).sum::<f64>() / values.len() as f64;
  let median = sorted[sorted.len() / 2] as f64;
  let p95 = sorted[(sorted.len() as f64 * 0.95) as usize] as f64;
  let p99 = sorted[(sorted.len() as f64 * 0.99) as usize] as f64;

  let variance = values
    .iter()
    .map(|&x| (x as f64 - mean).powi(2))
    .sum::<f64>()
    / values.len() as f64;
  let standard_deviation = variance.sqrt();

  DetailedMemoryStats {
    min,
    max,
    mean,
    median,
    p95,
    p99,
    standard_deviation,
    range: max - min,
  }
}

#[napi]
pub fn sample_native_memory() {
  if let Some(usage) = memory_stats() {
    SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
    let sample = MemorySample {
      physical_mem: usage.physical_mem as u64,
      virtual_mem: usage.virtual_mem as u64,
    };

    if let Ok(mut samples) = MEMORY_SAMPLES.lock() {
      samples.push(sample);
      // Keep only recent samples to prevent unbounded growth
      if samples.len() > 10000 {
        samples.drain(0..5000);
      }
    }
  }
}

#[napi]
pub fn get_native_memory_stats() -> Option<NativeMemoryStats> {
  if let Ok(samples) = MEMORY_SAMPLES.lock() {
    if samples.is_empty() {
      return None;
    }

    let physical_values: Vec<u64> = samples.iter().map(|s| s.physical_mem).collect();
    let virtual_values: Vec<u64> = samples.iter().map(|s| s.virtual_mem).collect();

    Some(NativeMemoryStats {
      physical_mem: calculate_detailed_stats(&physical_values),
      virtual_mem: calculate_detailed_stats(&virtual_values),
      sample_count: samples.len() as f64,
    })
  } else {
    None
  }
}

#[napi]
pub fn reset_memory_tracking() {
  if let Some(usage) = memory_stats() {
    BASELINE_PHYSICAL.store(usage.physical_mem as u64, Ordering::Relaxed);
    BASELINE_VIRTUAL.store(usage.virtual_mem as u64, Ordering::Relaxed);
    SAMPLE_COUNT.store(0, Ordering::Relaxed);

    if let Ok(mut samples) = MEMORY_SAMPLES.lock() {
      samples.clear();
    }
  }
}
