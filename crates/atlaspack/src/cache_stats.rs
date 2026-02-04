//! Cache statistics types for reporting native cache metrics.
//!
//! These types are used to return cache statistics from the Rust layer to JavaScript,
//! namespaced by cache user to support multiple caching systems.

use std::collections::HashMap;

use atlaspack_memoization_cache::StatsSnapshot;
use serde::Serialize;

/// Stats for transformer pipeline cache, including transformer-specific bailout breakdown.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformerPipelineCacheStats {
  pub hits: u64,
  pub misses: u64,
  pub uncacheables: u64,
  pub bailouts: u64,
  pub errors: u64,
  pub validations: u64,
  pub bailouts_by_transformer: HashMap<String, u64>,
}

impl TransformerPipelineCacheStats {
  pub fn from_snapshot(
    stats: &StatsSnapshot,
    bailouts_by_transformer: HashMap<String, u64>,
  ) -> Self {
    Self {
      hits: stats.hits,
      misses: stats.misses,
      uncacheables: stats.uncacheables,
      bailouts: stats.bailouts,
      errors: stats.errors,
      validations: stats.validations,
      bailouts_by_transformer,
    }
  }

  pub fn log(&self) {
    tracing::info!("Transformer pipeline cache stats: {:#?}", self);
  }
}

/// Top-level cache stats container, namespaced by cache user.
/// This allows different caching systems to report their stats independently.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCacheStats {
  pub transformer_pipelines: Option<TransformerPipelineCacheStats>,
}
