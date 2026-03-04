// Temporarily required becuase currently this request type is not used anywhere
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
  bundle_graph: Arc<RwLock<B>>,
  /// Maps each `HASH_REF_*` placeholder to the resolved name hash for that
  /// bundle. Provided by the parent `WriteBundlesRequest` which has already
  /// performed a topological sort so all dependencies are resolved before
  /// this request runs.
  pub hash_ref_to_name_hash: HashMap<String, String>,
  /// Raw bundle content used by the `.test` packager arm. Private so the
  /// varying struct shape is never visible outside this module.
  #[cfg(test)]
  test_content: Vec<u8>,
}

impl<B: BundleGraph + Send + Sync + 'static> PackageRequest<B> {
  /// Creates a new `PackageRequest`.
  pub fn new(
    bundle: Bundle,
    bundle_graph: Arc<RwLock<B>>,
    hash_ref_to_name_hash: HashMap<String, String>,
  ) -> Self {
    Self {
      bundle,
      bundle_graph,
      hash_ref_to_name_hash,
      #[cfg(test)]
      test_content: vec![],
    }
  }

  /// Creates a `PackageRequest` for use in tests using the `.test` file type.
  ///
  /// `content` is returned verbatim by the packager, bypassing any real
  /// packager implementation. This lets tests exercise hash substitution,
  /// file I/O and bundle naming without a real bundle graph or packager.
  #[cfg(test)]
  pub fn new_for_testing(
    bundle: Bundle,
    bundle_graph: Arc<RwLock<B>>,
    hash_ref_to_name_hash: HashMap<String, String>,
    content: Vec<u8>,
  ) -> Self {
    Self {
      bundle,
      bundle_graph,
      hash_ref_to_name_hash,
      test_content: content,
    }
  }
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
/// reference placeholder with the content hash produced by the packager.
///
/// The bundle's own hash reference is resolved from `content_hash` — the hash
/// of this bundle's packaged output. It cannot come from `hash_ref_to_name_hash`
/// because that map is built by the parent orchestrator *before* this bundle is
/// packaged; the current bundle's hash does not exist until packaging completes.
fn resolve_bundle_name(bundle: &Bundle, content_hash: &str) -> anyhow::Result<String> {
  let name = bundle
    .name
    .as_deref()
    .ok_or_else(|| anyhow!("Bundle has no name"))?;
  if !bundle.hash_reference.is_empty() && name.contains(&bundle.hash_reference) {
    Ok(name.replace(&bundle.hash_reference, content_hash))
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
      // To be able to unit test the stuff that happens after the file type packager runs, we implement this
      // test only file type that just returns the content it's given
      #[cfg(test)]
      FileType::Other(ref ext) if ext == ".test" => {
        use atlaspack_core::hash::hash_bytes;
        use atlaspack_core::package_result::BundleInfo;
        let content = self.test_content.clone();
        let hash = hash_bytes(&content);
        Ok(PackageResult {
          bundle_info: BundleInfo {
            bundle_type: ext.clone(),
            size: content.len() as u64,
            total_assets: 0,
            hash,
            hash_references: vec![],
            cache_keys: None,
            is_large_blob: false,
            time: None,
            bundle_contents: Some(content),
            map_contents: None,
          },
          config_requests: vec![],
          dev_dep_requests: vec![],
          invalidations: vec![],
        })
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

    // Resolve the output filename using the content hash from the packager result.
    // The bundle's own hash reference is derived from its content hash — it cannot
    // come from hash_ref_to_name_hash, which only contains hashes for *other* bundles
    // that this bundle's content references (resolved via topological ordering).
    let file_name = resolve_bundle_name(&self.bundle, &content_hash)?;
    let dist_dir = &self.bundle.target.dist_dir;
    let out_path = dist_dir.join(&file_name);

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
  use std::sync::Arc;

  use atlaspack_core::{
    bundle_graph::bundle_graph::BundleGraph,
    hash::hash_bytes,
    types::{Asset, Bundle, Dependency, Environment, Target},
  };
  use atlaspack_filesystem::FileSystem;
  use pretty_assertions::assert_eq;

  use crate::{
    request_tracker::{Request, RunRequestContext},
    requests::RequestResult,
    test_utils::{config_plugins, make_test_plugin_context},
  };

  use super::*;

  // ---------------------------------------------------------------------------
  // Minimal BundleGraph that satisfies the trait without any real data.
  // ---------------------------------------------------------------------------

  struct MockBundleGraph;

  impl BundleGraph for MockBundleGraph {
    fn get_bundles(&self) -> Vec<&Bundle> {
      vec![]
    }

    fn get_bundle_assets(&self, _bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
      Ok(vec![])
    }

    fn get_bundle_by_id(&self, _id: &str) -> Option<&Bundle> {
      None
    }

    fn get_public_asset_id(&self, _asset_id: &str) -> Option<&str> {
      None
    }

    fn get_dependencies(&self, _asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
      Ok(vec![])
    }

    fn get_resolved_asset(
      &self,
      _dependency: &Dependency,
      _bundle: &Bundle,
    ) -> anyhow::Result<Option<&Asset>> {
      Ok(None)
    }

    fn is_dependency_skipped(&self, _dependency: &Dependency) -> bool {
      false
    }
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

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

  fn test_bundle_type() -> FileType {
    FileType::Other(".test".to_string())
  }

  /// Build a `PackageRequest` using the `.test` file type with the given content
  /// and an optional pre-populated `hash_ref_to_name_hash` map.
  fn make_test_request(
    bundle: Bundle,
    content: &[u8],
    hash_ref_to_name_hash: HashMap<String, String>,
  ) -> PackageRequest<MockBundleGraph> {
    PackageRequest::new_for_testing(
      bundle,
      Arc::new(RwLock::new(MockBundleGraph)),
      hash_ref_to_name_hash,
      content.to_vec(),
    )
  }

  fn make_run_context() -> RunRequestContext {
    let ctx = make_test_plugin_context();
    let plugins = config_plugins(ctx);
    RunRequestContext::new_for_testing(plugins)
  }

  // ---------------------------------------------------------------------------
  // Unit tests for pure helper functions
  // ---------------------------------------------------------------------------

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
    let name = resolve_bundle_name(&bundle, "deadbeef").unwrap();
    assert_eq!(name, "index.deadbeef.js");
  }

  #[test]
  fn test_resolve_bundle_name_without_hash_reference() {
    let mut bundle = mock_bundle(FileType::Js);
    bundle.name = Some("index.js".to_string());
    let name = resolve_bundle_name(&bundle, "deadbeef").unwrap();
    assert_eq!(name, "index.js");
  }

  // ---------------------------------------------------------------------------
  // Integration tests exercising the full PackageRequest::run() path
  // ---------------------------------------------------------------------------

  /// Run a `PackageRequest` and return the output together with the FS so the
  /// caller can inspect files written during the run.
  async fn run_test_request(
    bundle: Bundle,
    content: &[u8],
    hash_ref_to_name_hash: HashMap<String, String>,
  ) -> (PackageRequestOutput, Arc<dyn FileSystem>) {
    let request = make_test_request(bundle, content, hash_ref_to_name_hash);
    let ctx = make_run_context();
    let fs = ctx.file_system().clone();
    let result = request.run(ctx).await.expect("PackageRequest::run failed");
    let output = match result.result {
      RequestResult::Package(o) => o,
      other => panic!("Expected RequestResult::Package, got {other:?}"),
    };
    (output, fs)
  }

  #[tokio::test]
  async fn test_run_writes_bundle_to_disk() {
    let content = b"bundle content";
    let dist_dir = PathBuf::from("/dist");
    let mut bundle = mock_bundle(test_bundle_type());
    bundle.name = Some("bundle.test".to_string());
    bundle.target = Target {
      dist_dir: dist_dir.clone(),
      ..Target::default()
    };

    let (output, fs) = run_test_request(bundle, content, HashMap::new()).await;

    let expected_path = dist_dir.join("bundle.test");
    assert_eq!(output.file_path, expected_path);
    assert_eq!(output.size, content.len() as u64);
    assert_eq!(
      fs.read(&expected_path).unwrap(),
      content.to_vec(),
      "file written to disk should match raw content"
    );
  }

  #[tokio::test]
  async fn test_run_hash_is_content_hash_of_raw_content() {
    let content = b"deterministic content";
    let mut bundle = mock_bundle(test_bundle_type());
    bundle.name = Some("out.test".to_string());
    bundle.target = Target {
      dist_dir: PathBuf::from("/dist"),
      ..Target::default()
    };

    let (output, _fs) = run_test_request(bundle, content, HashMap::new()).await;

    assert_eq!(output.hash, hash_bytes(content));
  }

  #[tokio::test]
  async fn test_run_substitutes_hash_refs_from_parent_map() {
    let placeholder = "HASH_REF_abcdef1234567890";
    let resolved = "cafebabe12345678";
    let content = format!("import '{placeholder}';").into_bytes();
    let expected_written = format!("import '{resolved}';").into_bytes();

    let mut map = HashMap::new();
    map.insert(placeholder.to_string(), resolved.to_string());

    let mut bundle = mock_bundle(test_bundle_type());
    bundle.name = Some("chunk.test".to_string());
    bundle.target = Target {
      dist_dir: PathBuf::from("/dist"),
      ..Target::default()
    };

    let (output, fs) = run_test_request(bundle, &content, map).await;

    assert_eq!(
      fs.read(&output.file_path).unwrap(),
      expected_written,
      "hash reference should be replaced in the written file"
    );
    assert_eq!(
      output.size,
      expected_written.len() as u64,
      "reported size should reflect substituted content"
    );
  }

  #[tokio::test]
  async fn test_run_resolves_bundle_name_from_own_hash() {
    let content = b"body";
    let content_hash = hash_bytes(content);
    let hash_ref = format!("HASH_REF_{:0<16}", "aa");
    let name_template = format!("index.{hash_ref}.test");

    let mut bundle = mock_bundle(test_bundle_type());
    bundle.hash_reference = hash_ref.clone();
    bundle.name = Some(name_template);
    bundle.target = Target {
      dist_dir: PathBuf::from("/dist"),
      ..Target::default()
    };

    let (output, fs) = run_test_request(bundle, content, HashMap::new()).await;

    let expected_name = format!("index.{content_hash}.test");
    let expected_path = PathBuf::from("/dist").join(&expected_name);
    assert_eq!(output.file_path, expected_path);
    assert!(
      fs.is_file(&expected_path),
      "file with resolved hash name should exist on disk"
    );
  }

  #[tokio::test]
  async fn test_run_errors_when_hash_ref_unresolved() {
    let content = b"HASH_REF_abcdef1234567890".to_vec();

    let mut bundle = mock_bundle(test_bundle_type());
    bundle.name = Some("out.test".to_string());
    bundle.target = Target {
      dist_dir: PathBuf::from("/dist"),
      ..Target::default()
    };

    let request = make_test_request(bundle, &content, HashMap::new());
    let ctx = make_run_context();
    let result = request.run(ctx).await;

    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("HASH_REF_abcdef1234567890")
    );
  }
}
