use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};
use napi_derive::napi;
use serde::Serialize;

/// Per-bundle info returned to JS after a full native build, mirroring the JS
/// `PackagedBundleInfo` type used to populate `bundleInfo` in `Atlaspack._build()`.
#[napi(object)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsPackagedBundleInfo {
  /// Absolute path to the written bundle file on disk.
  pub file_path: String,
  /// The bundle ID (same key used in the bundleInfo map).
  pub bundle_id: String,
  /// The bundle type (e.g. "js", "css").
  pub r#type: String,
  /// File size in bytes after hash substitution.
  pub size: u32,
  /// Wall-clock packaging time in milliseconds.
  pub time: u32,
}

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

/// Non-fatal diagnostic emitted by a native packager and forwarded to JS.
// TODO: Consolidate with a shared NAPI diagnostic type for `atlaspack_core::types::Diagnostic`.
// TODO: Forward `kind` (ErrorKind) when a serialisable representation is available.
#[derive(Debug, Clone)]
#[napi(object)]
pub struct PackagerDiagnostic {
  pub message: String,
  pub origin: Option<String>,
  pub hints: Vec<String>,
  pub documentation_url: Option<String>,
  pub name: Option<String>,
  pub code_highlights: Vec<PackagerCodeHighlight>,
}

/// A highlighted code region within a `PackagerDiagnostic`.
#[derive(Debug, Clone)]
#[napi(object)]
pub struct PackagerCodeHighlight {
  pub message: Option<String>,
  pub file_path: Option<String>,
  pub start_line: u32,
  pub start_col: u32,
  pub end_line: u32,
  pub end_col: u32,
}

#[napi(object)]
pub struct JsPackageResult {
  pub bundle_info: JsBundleInfo,
  pub config_requests: Vec<JsConfigRequest>,
  pub dev_dep_requests: Vec<JsDevDepRequest>,
  pub invalidations: Vec<JsRequestInvalidation>,
  pub warnings: Vec<PackagerDiagnostic>,
}

fn convert_package_diagnostic(diagnostic: atlaspack_core::types::Diagnostic) -> PackagerDiagnostic {
  PackagerDiagnostic {
    message: diagnostic.message,
    origin: diagnostic.origin,
    hints: diagnostic.hints,
    documentation_url: diagnostic.documentation_url,
    name: diagnostic.name,
    code_highlights: diagnostic
      .code_frames
      .into_iter()
      .flat_map(|frame| {
        let file_path = frame.file_path.map(|p| p.to_string_lossy().into_owned());
        frame
          .code_highlights
          .into_iter()
          .map(move |h| PackagerCodeHighlight {
            message: h.message,
            file_path: file_path.clone(),
            start_line: h.start.line as u32,
            start_col: h.start.column as u32,
            end_line: h.end.line as u32,
            end_col: h.end.column as u32,
          })
      })
      .collect(),
  }
}

// Conversion implementations
impl From<BundleInfo> for JsBundleInfo {
  fn from(info: BundleInfo) -> Self {
    JsBundleInfo {
      r#type: info.bundle_type,
      size: info.size as u32,
      hash: info.hash,
      hash_references: info.hash_references,
      cache_keys: info
        .cache_keys
        .expect("Cache keys are required when serializing to JS")
        .into(),
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
  fn from(value: PackageResult) -> Self {
    JsPackageResult {
      bundle_info: value.bundle_info.into(),
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
      warnings: value
        .warnings
        .into_iter()
        .map(convert_package_diagnostic)
        .collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};
  use atlaspack_core::types::DiagnosticBuilder;
  use pretty_assertions::assert_eq;

  use super::*;

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
      cache_keys: Some(rust_cache_keys),
      is_large_blob: false,
      time: Some(100),
      bundle_contents: None,
      map_contents: None,
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
      cache_keys: Some(rust_cache_keys),
      is_large_blob: true,
      time: Some(1000),
      bundle_contents: None,
      map_contents: None,
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
      cache_keys: Some(rust_cache_keys),
      is_large_blob: false,
      time: None,
      bundle_contents: None,
      map_contents: None,
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
      cache_keys: Some(rust_cache_keys),
      is_large_blob: false,
      time: Some(200),
      bundle_contents: None,
      map_contents: None,
    };

    let rust_result = PackageResult {
      bundle_info: rust_bundle_info,
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
      warnings: vec![],
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
      cache_keys: Some(rust_cache_keys),
      is_large_blob: false,
      time: None,
      bundle_contents: None,
      map_contents: None,
    };

    let rust_result = PackageResult {
      bundle_info: rust_bundle_info,
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
      warnings: vec![],
    };

    let js_result: JsPackageResult = rust_result.into();

    assert_eq!(
      js_result.bundle_info.hash_references,
      vec!["ref1", "ref2", "ref3"]
    );
  }

  #[test]
  fn package_result_warnings_are_forwarded() {
    let result = PackageResult {
      bundle_info: BundleInfo {
        bundle_type: "css".to_string(),
        size: 0,
        total_assets: 0,
        hash: String::new(),
        hash_references: vec![],
        cache_keys: Some(CacheKeyMap {
          content: "c".to_string(),
          map: "m".to_string(),
          info: "i".to_string(),
        }),
        is_large_blob: false,
        time: None,
        bundle_contents: None,
        map_contents: None,
      },
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
      warnings: vec![
        DiagnosticBuilder::default()
          .message("unused import".to_string())
          .origin(Some("css-packager".to_string()))
          .hints(vec!["remove it".to_string()])
          .name(Some("UnusedImport".to_string()))
          .build()
          .unwrap(),
      ],
    };

    let js_result = JsPackageResult::from(result);

    assert_eq!(js_result.warnings.len(), 1);
    assert_eq!(js_result.warnings[0].message, "unused import");
    assert_eq!(
      js_result.warnings[0].origin,
      Some("css-packager".to_string())
    );
    assert_eq!(js_result.warnings[0].hints, vec!["remove it".to_string()]);
    assert_eq!(js_result.warnings[0].name, Some("UnusedImport".to_string()));
    assert_eq!(js_result.warnings[0].documentation_url, None);
    assert!(js_result.warnings[0].code_highlights.is_empty());
  }

  #[test]
  fn convert_package_diagnostic_maps_code_frames_to_highlights() {
    use atlaspack_core::types::{CodeFrame, CodeHighlight, Location};

    let diagnostic = atlaspack_core::types::DiagnosticBuilder::default()
      .message("syntax error".to_string())
      .origin(Some("css-packager".to_string()))
      .code_frames(vec![CodeFrame {
        file_path: Some(std::path::PathBuf::from("/src/styles.css")),
        code: None,
        code_highlights: vec![
          CodeHighlight {
            message: Some("unexpected token".to_string()),
            start: Location {
              line: 10,
              column: 5,
            },
            end: Location {
              line: 10,
              column: 12,
            },
          },
          CodeHighlight {
            message: Some("hint here".to_string()),
            start: Location {
              line: 15,
              column: 1,
            },
            end: Location {
              line: 16,
              column: 3,
            },
          },
        ],
        language: None,
      }])
      .build()
      .unwrap();

    let result = PackageResult {
      bundle_info: BundleInfo {
        bundle_type: "css".to_string(),
        size: 0,
        total_assets: 0,
        hash: String::new(),
        hash_references: vec![],
        cache_keys: Some(CacheKeyMap {
          content: "c".to_string(),
          map: "m".to_string(),
          info: "i".to_string(),
        }),
        is_large_blob: false,
        time: None,
        bundle_contents: None,
        map_contents: None,
      },
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
      warnings: vec![diagnostic],
    };

    let js_result = JsPackageResult::from(result);

    assert_eq!(js_result.warnings.len(), 1);
    let highlights = &js_result.warnings[0].code_highlights;
    assert_eq!(highlights.len(), 2);

    assert_eq!(highlights[0].message, Some("unexpected token".to_string()));
    assert_eq!(highlights[0].file_path, Some("/src/styles.css".to_string()));
    assert_eq!(highlights[0].start_line, 10);
    assert_eq!(highlights[0].start_col, 5);
    assert_eq!(highlights[0].end_line, 10);
    assert_eq!(highlights[0].end_col, 12);

    assert_eq!(highlights[1].message, Some("hint here".to_string()));
    assert_eq!(highlights[1].start_line, 15);
    assert_eq!(highlights[1].end_line, 16);
    assert_eq!(highlights[1].end_col, 3);
  }

  #[test]
  #[should_panic(expected = "Cache keys are required")]
  fn bundle_info_from_panics_when_cache_keys_is_none() {
    let info = BundleInfo {
      bundle_type: "js".to_string(),
      size: 100,
      total_assets: 1,
      hash: "h".to_string(),
      hash_references: vec![],
      cache_keys: None, // intentionally None — should panic
      is_large_blob: false,
      time: None,
      bundle_contents: None,
      map_contents: None,
    };
    let _: JsBundleInfo = info.into(); // must panic
  }
}
