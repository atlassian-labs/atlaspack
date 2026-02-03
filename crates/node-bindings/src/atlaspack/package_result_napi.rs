use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};
use napi_derive::napi;

/// NAPI-compatible BundleInfo
#[napi(object)]
pub struct JsBundleInfo {
  pub r#type: String, // "type" is reserved in JS, use r#type
  pub size: u32,      // NAPI prefers u32 over u64 for numbers
  pub hash: String,
  pub hash_references: Vec<String>,
  pub cache_keys: JsCacheKeyMap,
  pub is_large_blob: bool,
  pub time: Option<u32>,
}

#[napi(object)]
pub struct JsCacheKeyMap {
  pub content: String,
  pub map: String,
  pub info: String,
}

#[napi(object)]
pub struct JsRequestInvalidation {
  pub r#type: String, // "file" | "env" | "option"
  pub file_path: Option<String>,
  pub key: Option<String>,
}

#[napi(object)]
pub struct JsConfigRequest {
  pub id: String,
  pub invalidate_on_file_change: Vec<String>,
  pub invalidate_on_file_create: Vec<JsRequestInvalidation>,
  pub invalidate_on_env_change: Vec<String>,
  pub invalidate_on_option_change: Vec<String>,
  pub invalidate_on_startup: bool,
  pub invalidate_on_build: bool,
}

#[napi(object)]
pub struct JsDevDepRequest {
  pub specifier: String,
  pub resolve_from: String,
  pub hash: String,
}

#[napi(object)]
pub struct JsPackageResult {
  pub bundle_info: JsBundleInfo,
  pub config_requests: Vec<JsConfigRequest>,
  pub dev_dep_requests: Vec<JsDevDepRequest>,
  pub invalidations: Vec<JsRequestInvalidation>,
}

// Conversion implementations
impl From<BundleInfo> for JsBundleInfo {
  fn from(info: BundleInfo) -> Self {
    JsBundleInfo {
      r#type: info.bundle_type,
      size: info.size as u32,
      hash: info.hash,
      hash_references: info.hash_references,
      cache_keys: info.cache_keys.into(),
      is_large_blob: info.is_large_blob,
      time: info.time.map(|t| t as u32),
    }
  }
}

impl From<CacheKeyMap> for JsCacheKeyMap {
  fn from(keys: CacheKeyMap) -> Self {
    JsCacheKeyMap {
      content: keys.content,
      map: keys.map,
      info: keys.info,
    }
  }
}

// impl From<RequestInvalidation> for JsRequestInvalidation {
//   fn from(inv: RequestInvalidation) -> Self {
//     match inv {
//       RequestInvalidation::File { file_path } => JsRequestInvalidation {
//         r#type: "file".to_string(),
//         file_path: Some(file_path),
//         key: None,
//       },
//       RequestInvalidation::Env { key } => JsRequestInvalidation {
//         r#type: "env".to_string(),
//         file_path: None,
//         key: Some(key),
//       },
//       RequestInvalidation::Option { key } => JsRequestInvalidation {
//         r#type: "option".to_string(),
//         file_path: None,
//         key: Some(key),
//       },
//     }
//   }
// }

impl From<PackageResult> for JsPackageResult {
  fn from(result: PackageResult) -> Self {
    JsPackageResult {
      bundle_info: result.bundle_info.into(),
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_bundle_info_conversion() {
    let rust_cache_keys = CacheKeyMap {
      content: "content-key".to_string(),
      map: "map-key".to_string(),
      info: "info-key".to_string(),
    };

    let rust_bundle_info = BundleInfo {
      bundle_type: "js".to_string(),
      size: 1024,
      total_assets: 5,
      hash: "abc123".to_string(),
      hash_references: vec!["ref1".to_string()],
      cache_keys: rust_cache_keys,
      is_large_blob: false,
      time: Some(100),
    };

    let js_bundle_info: JsBundleInfo = rust_bundle_info.into();

    assert_eq!(js_bundle_info.r#type, "js");
    assert_eq!(js_bundle_info.size, 1024);
    assert_eq!(js_bundle_info.hash, "abc123");
    assert_eq!(js_bundle_info.hash_references.len(), 1);
    assert_eq!(js_bundle_info.cache_keys.content, "content-key");
    assert!(!js_bundle_info.is_large_blob);
    assert_eq!(js_bundle_info.time, Some(100));
  }

  #[test]
  fn test_bundle_info_conversion_large_size() {
    let rust_cache_keys = CacheKeyMap {
      content: "c".to_string(),
      map: "m".to_string(),
      info: "i".to_string(),
    };

    // Test with u64 value larger than u32::MAX
    let rust_bundle_info = BundleInfo {
      bundle_type: "js".to_string(),
      size: 5_000_000_000, // > u32::MAX
      total_assets: 100,
      hash: "hash".to_string(),
      hash_references: vec![],
      cache_keys: rust_cache_keys,
      is_large_blob: true,
      time: Some(1000),
    };

    let js_bundle_info: JsBundleInfo = rust_bundle_info.into();

    // u64 to u32 conversion will truncate
    assert_eq!(js_bundle_info.size, 5_000_000_000u64 as u32);
    assert!(js_bundle_info.is_large_blob);
  }

  #[test]
  fn test_bundle_info_conversion_no_time() {
    let rust_cache_keys = CacheKeyMap {
      content: "c".to_string(),
      map: "m".to_string(),
      info: "i".to_string(),
    };

    let rust_bundle_info = BundleInfo {
      bundle_type: "css".to_string(),
      size: 512,
      total_assets: 2,
      hash: "def456".to_string(),
      hash_references: vec![],
      cache_keys: rust_cache_keys,
      is_large_blob: false,
      time: None,
    };

    let js_bundle_info: JsBundleInfo = rust_bundle_info.into();

    assert_eq!(js_bundle_info.time, None);
  }

  #[test]
  fn test_cache_key_map_conversion() {
    let rust_cache_keys = CacheKeyMap {
      content: "PackagerRunner/1.0.0/abc/content".to_string(),
      map: "PackagerRunner/1.0.0/abc/map".to_string(),
      info: "PackagerRunner/1.0.0/abc/info".to_string(),
    };

    let js_cache_keys: JsCacheKeyMap = rust_cache_keys.into();

    assert_eq!(js_cache_keys.content, "PackagerRunner/1.0.0/abc/content");
    assert_eq!(js_cache_keys.map, "PackagerRunner/1.0.0/abc/map");
    assert_eq!(js_cache_keys.info, "PackagerRunner/1.0.0/abc/info");
  }

  #[test]
  fn test_package_result_conversion() {
    let rust_cache_keys = CacheKeyMap {
      content: "content".to_string(),
      map: "map".to_string(),
      info: "info".to_string(),
    };

    let rust_bundle_info = BundleInfo {
      bundle_type: "js".to_string(),
      size: 2048,
      total_assets: 10,
      hash: "xyz789".to_string(),
      hash_references: vec!["ref1".to_string(), "ref2".to_string()],
      cache_keys: rust_cache_keys,
      is_large_blob: false,
      time: Some(200),
    };

    let rust_result = PackageResult {
      bundle_info: rust_bundle_info,
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
    };

    let js_result: JsPackageResult = rust_result.into();

    assert_eq!(js_result.bundle_info.r#type, "js");
    assert_eq!(js_result.bundle_info.size, 2048);
    assert_eq!(js_result.bundle_info.hash, "xyz789");
    assert!(js_result.config_requests.is_empty());
    assert!(js_result.dev_dep_requests.is_empty());
    assert!(js_result.invalidations.is_empty());
  }

  #[test]
  fn test_package_result_preserves_hash_references() {
    let rust_cache_keys = CacheKeyMap {
      content: "c".to_string(),
      map: "m".to_string(),
      info: "i".to_string(),
    };

    let rust_bundle_info = BundleInfo {
      bundle_type: "js".to_string(),
      size: 100,
      total_assets: 1,
      hash: "hash".to_string(),
      hash_references: vec!["ref1".to_string(), "ref2".to_string(), "ref3".to_string()],
      cache_keys: rust_cache_keys,
      is_large_blob: false,
      time: None,
    };

    let rust_result = PackageResult {
      bundle_info: rust_bundle_info,
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
    };

    let js_result: JsPackageResult = rust_result.into();

    assert_eq!(js_result.bundle_info.hash_references.len(), 3);
    assert_eq!(js_result.bundle_info.hash_references[0], "ref1");
    assert_eq!(js_result.bundle_info.hash_references[1], "ref2");
    assert_eq!(js_result.bundle_info.hash_references[2], "ref3");
  }
}
