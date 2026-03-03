#![allow(dead_code)]

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph,
  debug_tools::DebugTools,
  package_result::PackageResult,
  types::{Bundle, FileType},
};
use atlaspack_packager_js::{JsPackager, PackagingContext};
use parking_lot::RwLock;

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::RequestResult,
};

/// The prefix used in hash reference placeholders embedded in bundle content.
/// Matches `HASH_REF_PREFIX` in `packages/core/core/src/constants.ts`.
const HASH_REF_PREFIX: &str = "HASH_REF_";
/// Number of hex characters that make up the hash portion of a hash reference.
/// Matches `HASH_REF_HASH_LEN` in `packages/core/core/src/constants.ts`.
const HASH_REF_HASH_LEN: usize = 16;
/// Total length of a hash reference placeholder: prefix + hash chars.
const HASH_REF_PLACEHOLDER_LEN: usize = HASH_REF_PREFIX.len() + HASH_REF_HASH_LEN;

/// Output produced by a successfully packaged bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageRequestOutput {
  /// Absolute path where the bundle was written on disk.
  pub file_path: PathBuf,
  /// Size of the written bundle in bytes (after hash substitution).
  pub size: u64,
  /// Content hash of the packaged bundle (before hash substitution).
  pub hash: String,
  /// Packaging time in milliseconds.
  pub time: u64,
}

/// A request that packages a single bundle: runs the packager, performs hash
/// substitution using a pre-computed map (resolved by the parent orchestrator
/// via topological sort of the bundle graph), and writes the result to disk.
pub struct PackageRequest<B: BundleGraph + Send + Sync + 'static> {
  pub bundle: Bundle,
  pub bundle_graph: Arc<RwLock<B>>,
  /// Maps each `HASH_REF_*` placeholder to the resolved name hash for that
  /// bundle. Provided by the parent `WriteBundlesRequest` which has already
  /// performed a topological sort so all dependencies are resolved before
  /// this request runs.
  pub hash_ref_to_name_hash: HashMap<String, String>,
}

impl<B: BundleGraph + Send + Sync + 'static> std::fmt::Debug for PackageRequest<B> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PackageRequest")
      .field("bundle", &self.bundle)
      .finish_non_exhaustive()
  }
}

impl<B: BundleGraph + Send + Sync + 'static> Hash for PackageRequest<B> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.bundle.hash(state);
  }
}

/// Replace all `HASH_REF_*` placeholders in `content` using `hash_ref_to_name_hash`.
/// Returns an error if any placeholder in the content has no corresponding entry in the map,
/// since the parent orchestrator is expected to have resolved all hashes before this runs.
fn apply_hash_substitution(
  content: Vec<u8>,
  hash_ref_to_name_hash: &HashMap<String, String>,
) -> anyhow::Result<Vec<u8>> {
  let prefix = HASH_REF_PREFIX.as_bytes();
  let mut result = Vec::with_capacity(content.len());
  let mut i = 0;
  while i < content.len() {
    if i + HASH_REF_PLACEHOLDER_LEN <= content.len() && content[i..].starts_with(prefix) {
      let placeholder = &content[i..i + HASH_REF_PLACEHOLDER_LEN];
      if placeholder[prefix.len()..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
      {
        let key = String::from_utf8_lossy(placeholder).into_owned();
        let name_hash = hash_ref_to_name_hash
          .get(&key)
          .ok_or_else(|| anyhow!("No hash resolution found for placeholder '{key}'. The parent orchestrator must resolve all hash references before running PackageRequest."))?;
        result.extend_from_slice(name_hash.as_bytes());
        i += HASH_REF_PLACEHOLDER_LEN;
        continue;
      }
    }
    result.push(content[i]);
    i += 1;
  }
  Ok(result)
}

/// Derive the output filename for a bundle by substituting its own hash
/// reference placeholder in the bundle name using `hash_ref_to_name_hash`.
fn resolve_bundle_name(
  bundle: &Bundle,
  hash_ref_to_name_hash: &HashMap<String, String>,
) -> anyhow::Result<String> {
  let name = bundle
    .name
    .as_deref()
    .ok_or_else(|| anyhow!("Bundle has no name"))?;
  if !bundle.hash_reference.is_empty() && name.contains(&bundle.hash_reference) {
    let name_hash = hash_ref_to_name_hash
      .get(&bundle.hash_reference)
      .ok_or_else(|| {
        anyhow!(
          "No hash resolution found for bundle hash reference '{}'. \
           The parent orchestrator must resolve all hash references before running PackageRequest.",
          bundle.hash_reference
        )
      })?;
    Ok(name.replace(&bundle.hash_reference, name_hash))
  } else {
    Ok(name.to_string())
  }
}

#[async_trait]
impl<B: BundleGraph + Send + Sync + 'static> Request for PackageRequest<B> {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let start = Instant::now();

    let package_result: Result<PackageResult, RunRequestError> = match self.bundle.bundle_type {
      FileType::Js => {
        let packager = JsPackager::new(
          PackagingContext {
            db: Arc::clone(&request_context.db),
            cache: None,
            project_root: request_context.project_root.clone(),
            debug_tools: DebugTools::default(),
          },
          Arc::clone(&self.bundle_graph),
        );
        packager.package(&self.bundle.id)
      }
      _ => Err(anyhow!(
        "Unsupported bundle type: {:?}",
        self.bundle.bundle_type
      )),
    };

    let package_result = package_result?;
    let bundle_info = package_result.bundle_info;
    let content_hash = bundle_info.hash;

    // Retrieve the raw bundle content bytes
    let raw_contents = bundle_info
      .bundle_contents
      .ok_or_else(|| anyhow!("Bundle contents are required when packaging"))?;

    // Apply hash substitution. All placeholders must be resolvable — the
    // parent orchestrator (WriteBundlesRequest) performs a topological sort
    // of the bundle graph and ensures this map is complete before running
    // each PackageRequest.
    let substituted_contents = apply_hash_substitution(raw_contents, &self.hash_ref_to_name_hash)?;

    // Resolve the output filename using the same map.
    let file_name = resolve_bundle_name(&self.bundle, &self.hash_ref_to_name_hash)?;
    let dist_dir = &self.bundle.target.dist_dir;
    let out_path = dist_dir.join(&file_name);

    // Write to disk via the request context file system (not std::fs) so
    // that in-memory or other FS implementations work correctly in tests.
    let fs = request_context.file_system();
    fs.create_dir_all(dist_dir)
      .map_err(|e| anyhow!("Failed to create output directory {:?}: {}", dist_dir, e))?;
    fs.write(&out_path, &substituted_contents)
      .map_err(|e| anyhow!("Failed to write bundle to {:?}: {}", out_path, e))?;

    let size = substituted_contents.len() as u64;
    let time_ms = start.elapsed().as_millis() as u64;

    Ok(ResultAndInvalidations {
      result: RequestResult::Package(PackageRequestOutput {
        file_path: out_path,
        size,
        hash: content_hash,
        time: time_ms,
      }),
      invalidations: vec![],
    })
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_core::types::{Environment, Target};
  use pretty_assertions::assert_eq;

  use super::*;

  fn mock_bundle(bundle_type: FileType) -> Bundle {
    Bundle {
      bundle_behavior: None,
      bundle_type,
      entry_asset_ids: vec![],
      env: Environment::default(),
      hash_reference: String::new(),
      id: String::new(),
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: None,
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      target: Target::default(),
    }
  }

  #[test]
  fn test_apply_hash_substitution_no_placeholders() {
    let content = b"hello world".to_vec();
    let map = HashMap::new();
    let result = apply_hash_substitution(content.clone(), &map).unwrap();
    assert_eq!(result, content);
  }

  #[test]
  fn test_apply_hash_substitution_replaces() {
    let content = b"prefix HASH_REF_abcdef1234567890 suffix".to_vec();
    let mut map = HashMap::new();
    map.insert(
      "HASH_REF_abcdef1234567890".to_string(),
      "deadbeef".to_string(),
    );
    let result = apply_hash_substitution(content, &map).unwrap();
    assert_eq!(result, b"prefix deadbeef suffix".to_vec());
  }

  #[test]
  fn test_apply_hash_substitution_replaces_multiple() {
    let content = b"HASH_REF_abcdef1234567890 and HASH_REF_0987654321fedcba".to_vec();
    let mut map = HashMap::new();
    map.insert(
      "HASH_REF_abcdef1234567890".to_string(),
      "aaaaaaaa".to_string(),
    );
    map.insert(
      "HASH_REF_0987654321fedcba".to_string(),
      "bbbbbbbb".to_string(),
    );
    let result = apply_hash_substitution(content, &map).unwrap();
    assert_eq!(result, b"aaaaaaaa and bbbbbbbb".to_vec());
  }

  #[test]
  fn test_apply_hash_substitution_errors_on_missing_ref() {
    let content = b"HASH_REF_abcdef1234567890".to_vec();
    let map = HashMap::new();
    let result = apply_hash_substitution(content, &map);
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("HASH_REF_abcdef1234567890")
    );
  }

  #[test]
  fn test_resolve_bundle_name_with_hash_reference() {
    let mut bundle = mock_bundle(FileType::Js);
    bundle.hash_reference = "HASH_REF_abcdef1234567890".to_string();
    bundle.name = Some("index.HASH_REF_abcdef1234567890.js".to_string());
    let mut map = HashMap::new();
    map.insert(
      "HASH_REF_abcdef1234567890".to_string(),
      "deadbeef".to_string(),
    );
    let name = resolve_bundle_name(&bundle, &map).unwrap();
    assert_eq!(name, "index.deadbeef.js");
  }

  #[test]
  fn test_resolve_bundle_name_without_hash_reference() {
    let mut bundle = mock_bundle(FileType::Js);
    bundle.name = Some("index.js".to_string());
    let name = resolve_bundle_name(&bundle, &HashMap::new()).unwrap();
    assert_eq!(name, "index.js");
  }

  #[test]
  fn test_resolve_bundle_name_errors_on_missing_ref() {
    let mut bundle = mock_bundle(FileType::Js);
    bundle.hash_reference = "HASH_REF_abcdef1234567890".to_string();
    bundle.name = Some("index.HASH_REF_abcdef1234567890.js".to_string());
    let result = resolve_bundle_name(&bundle, &HashMap::new());
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("HASH_REF_abcdef1234567890")
    );
  }
}
