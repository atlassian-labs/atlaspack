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
