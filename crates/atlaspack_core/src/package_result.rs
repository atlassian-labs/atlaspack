// The result of packaging a bundle
#[derive(Debug, Clone)]
pub struct PackageResult {
  pub bundle_info: BundleInfo,
  pub config_requests: Vec<()>, // TODO only here for compat with JS orchestrator
  pub dev_dep_requests: Vec<()>, // TODO only here for compat with JS orchestrator
  pub invalidations: Vec<()>,   // TODO only here for compat with JS orchestrator
}

#[derive(Debug, Clone)]
pub struct BundleInfo {
  pub bundle_type: String,
  pub size: u64,
  pub total_assets: u64,
  pub hash: String,
  pub hash_references: Vec<String>,
  pub cache_keys: CacheKeyMap,
  pub is_large_blob: bool,
  pub time: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CacheKeyMap {
  pub content: String,
  pub map: String,
  pub info: String,
}
