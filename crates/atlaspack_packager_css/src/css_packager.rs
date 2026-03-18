use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::package_result::{BundleInfo, PackageResult};
use atlaspack_core::types::Priority;
use lightningcss::bundler::{Bundler, SourceProvider};
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;

use crate::{CssPackager, CssPackagingContext};

/// Stores CSS strings in a Vec (indexed by HashMap) so that `read<'a>` can
/// return `&'a str` tied to `&'a self` without unsafe code.
struct InMemoryCssProvider {
  strings: Vec<String>,
  index: HashMap<PathBuf, usize>,
}

impl InMemoryCssProvider {
  fn new(map: HashMap<String, String>) -> Self {
    let mut strings = Vec::with_capacity(map.len());
    let mut index = HashMap::with_capacity(map.len());
    for (key, val) in map {
      let idx = strings.len();
      strings.push(val);
      index.insert(PathBuf::from(&key), idx);
    }
    Self { strings, index }
  }
}

impl SourceProvider for InMemoryCssProvider {
  type Error = std::io::Error;

  fn read<'a>(&'a self, file: &Path) -> Result<&'a str, Self::Error> {
    // Return empty string for unknown files (e.g. external URLs not in the map).
    Ok(
      self
        .index
        .get(file)
        .map(|&idx| self.strings[idx].as_str())
        .unwrap_or(""),
    )
  }

  fn resolve(&self, specifier: &str, _originating_file: &Path) -> Result<PathBuf, Self::Error> {
    // Identity resolution — treat specifier as the path key directly.
    Ok(PathBuf::from(specifier))
  }
}

impl<B: BundleGraph + Send + Sync> CssPackager<B> {
  pub fn new(context: CssPackagingContext, bundle_graph: Arc<B>) -> Self {
    Self {
      context,
      bundle_graph,
    }
  }

  /// Packages a CSS bundle by:
  /// 1. Collecting assets in source order from the bundle graph.
  /// 2. Building a synthetic `@import`-based entry that drives lightningcss bundling.
  /// 3. Hoisting unresolvable external `@import`s (e.g. Google Fonts URLs) to the top.
  /// 4. Returning the final CSS as `PackageResult::bundle_contents`.
  pub fn package(&self, bundle_id: &str) -> Result<PackageResult> {
    let bundle = self
      .bundle_graph
      .get_bundle_by_id(bundle_id)
      .ok_or_else(|| anyhow::anyhow!("Bundle not found: {bundle_id}"))?;

    let assets = self
      .bundle_graph
      .get_bundle_assets_in_source_order(bundle)?;

    // Phase 1: build synthetic entry, collect hoisted external imports, and populate CSS map.
    let mut hoisted_imports: Vec<String> = Vec::new();
    // Specifiers already identified as external (for stripping from asset CSS before bundling).
    let mut external_specifiers: Vec<String> = Vec::new();
    let mut entry_contents = String::new();
    let mut css_code_map: HashMap<String, String> = HashMap::new();

    for asset in &assets {
      // Emit a synthetic @import for this asset in the bundle entry.
      entry_contents.push_str(&format!("@import \"{}\";\n", asset.id));

      // Identify unresolvable Sync deps — these are external @imports to hoist.
      let deps = self.bundle_graph.get_dependencies(asset)?;
      for dep in deps {
        if dep.priority != Priority::Sync {
          continue;
        }
        if self.bundle_graph.is_dependency_skipped(dep) {
          continue;
        }
        let resolved = self.bundle_graph.get_resolved_asset(dep, bundle)?;
        if resolved.is_none() {
          hoisted_imports.push(format!("@import \"{}\";", dep.specifier));
          external_specifiers.push(dep.specifier.clone());
        }
      }

      // Read CSS content from the database (fall back to asset.id as the key).
      let db_key = asset.content_key.as_deref().unwrap_or(&asset.id);
      let css_bytes = self.context.db.get(db_key)?.unwrap_or_default();
      let css_code = String::from_utf8(css_bytes)
        .map_err(|e| anyhow::anyhow!("Asset {} CSS is not valid UTF-8: {e}", asset.id))?;

      // Strip external @import lines from the asset CSS before handing to the Bundler.
      // This prevents the Bundler from encountering unresolvable URLs, which could error
      // or produce duplicate imports alongside our manually hoisted ones.
      let filtered_css = if external_specifiers.is_empty() {
        css_code
      } else {
        filter_external_imports(&css_code, &external_specifiers)
      };

      css_code_map.insert(asset.id.clone(), filtered_css);
    }

    // Phase 2: insert the synthetic entry under a unique key to avoid collision with asset IDs.
    // We use a reserved prefix that is unlikely to match any real asset ID or file path.
    let entry_path = format!("__atlaspack_entry_{}.css", bundle_id);
    css_code_map.insert(entry_path.clone(), entry_contents);

    // Phase 3: bundle via lightningcss::Bundler — resolves all internal @imports.
    let provider = InMemoryCssProvider::new(css_code_map);
    let mut bundler = Bundler::new(&provider, None, ParserOptions::default());
    let stylesheet = bundler
      .bundle(Path::new(&entry_path))
      .map_err(|e| anyhow::anyhow!("lightningcss bundling failed: {:?}", e))?;
    let result = stylesheet
      .to_css(PrinterOptions::default())
      .map_err(|e| anyhow::anyhow!("lightningcss printing failed: {:?}", e))?;
    let mut css = result.code;

    // Phase 4: prepend hoisted external @imports before all inlined rules.
    if !hoisted_imports.is_empty() {
      let hoisted = hoisted_imports.join("\n");
      css = format!("{hoisted}\n{css}");
    }

    let size = css.len() as u64;
    Ok(PackageResult {
      bundle_info: BundleInfo {
        bundle_type: "css".to_string(),
        size,
        total_assets: assets.len() as u64,
        hash: String::new(),
        hash_references: vec![],
        cache_keys: None,
        is_large_blob: false,
        time: None,
        bundle_contents: Some(css.into_bytes()),
        map_contents: None,
      },
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
    })
  }
}

/// Removes lines from `css` that contain an `@import` for any of the given external specifiers.
/// This prevents the Bundler from encountering unresolvable URLs and producing errors or duplicates.
fn filter_external_imports(css: &str, external_specifiers: &[String]) -> String {
  css
    .lines()
    .filter(|line| {
      let trimmed = line.trim();
      if !trimmed.starts_with("@import") {
        return true;
      }
      // Keep the line only if it does NOT reference any known external specifier.
      !external_specifiers
        .iter()
        .any(|spec| trimmed.contains(spec.as_str()))
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[cfg(test)]
mod tests {
  use std::collections::{HashMap, HashSet};
  use std::path::PathBuf;
  use std::sync::Arc;

  use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
  use atlaspack_core::database::{DatabaseRef, InMemoryDatabase};
  use atlaspack_core::package_result::PackageResult;
  use atlaspack_core::types::{
    Asset, Bundle, Dependency, DependencyBuilder, Environment, FileType, Priority, SpecifierType,
    Target,
  };

  use super::*;

  // ---------------------------------------------------------------------------
  // TestBundleGraph — manual BundleGraph mock
  // ---------------------------------------------------------------------------

  struct TestBundleGraph {
    /// All bundles in this graph.
    bundles: Vec<Bundle>,
    /// bundle_id → assets in source order (insertion order = expected output order).
    assets_by_bundle: HashMap<String, Vec<Asset>>,
    /// asset_id → outgoing dependencies.
    deps_by_asset: HashMap<String, Vec<Dependency>>,
    /// dep specifier or placeholder → resolved Asset (internal imports).
    resolved: HashMap<String, Asset>,
    /// dep specifiers that are marked as skipped (tree-shaken away).
    skipped: HashSet<String>,
  }

  impl TestBundleGraph {
    fn new() -> Self {
      Self {
        bundles: Vec::new(),
        assets_by_bundle: HashMap::new(),
        deps_by_asset: HashMap::new(),
        resolved: HashMap::new(),
        skipped: HashSet::new(),
      }
    }
  }

  impl BundleGraph for TestBundleGraph {
    fn get_bundles(&self) -> Vec<&Bundle> {
      self.bundles.iter().collect()
    }

    fn get_bundle_assets(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
      Ok(
        self
          .assets_by_bundle
          .get(&bundle.id)
          .map(|v| v.iter().collect())
          .unwrap_or_default(),
      )
    }

    fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle> {
      self.bundles.iter().find(|b| b.id == id)
    }

    fn get_public_asset_id(&self, _asset_id: &str) -> Option<&str> {
      None
    }

    fn get_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
      Ok(
        self
          .deps_by_asset
          .get(&asset.id)
          .map(|v| v.iter().collect())
          .unwrap_or_default(),
      )
    }

    fn get_resolved_asset(
      &self,
      dependency: &Dependency,
      _bundle: &Bundle,
    ) -> anyhow::Result<Option<&Asset>> {
      let key = dependency
        .placeholder
        .as_deref()
        .unwrap_or(dependency.specifier.as_str());
      Ok(self.resolved.get(key))
    }

    fn is_dependency_skipped(&self, dependency: &Dependency) -> bool {
      self.skipped.contains(dependency.specifier.as_str())
    }

    fn get_incoming_dependencies(&self, _asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
      Ok(vec![])
    }

    /// Returns assets in the insertion order recorded in `assets_by_bundle`.
    fn get_bundle_assets_in_source_order(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
      self.get_bundle_assets(bundle)
    }

    fn get_referenced_bundle_ids(&self, _bundle: &Bundle) -> Vec<String> {
      vec![]
    }

    fn get_inline_bundle_ids(&self, _bundle: &Bundle) -> Vec<String> {
      vec![]
    }
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  fn make_asset(id: &str) -> Asset {
    Asset {
      id: id.to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    }
  }

  /// Creates an asset whose DB content is keyed by `content_key` rather than `id`.
  /// Used to verify the packager falls back correctly when `content_key` is set.
  #[allow(dead_code)]
  fn make_asset_with_content_key(id: &str, content_key: &str) -> Asset {
    Asset {
      id: id.to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      content_key: Some(content_key.to_string()),
      ..Asset::default()
    }
  }

  fn make_bundle(id: &str, entry_asset_ids: Vec<&str>) -> Bundle {
    Bundle {
      id: id.to_string(),
      bundle_type: FileType::Css,
      entry_asset_ids: entry_asset_ids.iter().map(|s| s.to_string()).collect(),
      env: Environment::default(),
      hash_reference: String::new(),
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: None,
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      bundle_behavior: None,
      is_placeholder: false,
      target: Target::default(),
    }
  }

  fn make_dependency(specifier: &str, priority: Priority) -> Dependency {
    DependencyBuilder::default()
      .specifier(specifier.to_string())
      .specifier_type(SpecifierType::Url)
      .priority(priority)
      .env(Arc::new(Environment::default()))
      .build()
  }

  fn make_db() -> DatabaseRef {
    Arc::new(InMemoryDatabase::default()) as DatabaseRef
  }

  fn output_string(result: &PackageResult) -> String {
    let bytes = result
      .bundle_info
      .bundle_contents
      .as_ref()
      .expect("bundle_contents must be Some");
    String::from_utf8(bytes.clone()).expect("output must be valid UTF-8")
  }

  // ---------------------------------------------------------------------------
  // Test 5 (existing test updated): CssPackagingContext now requires a db field
  // ---------------------------------------------------------------------------

  #[test]
  fn css_packaging_context_fields_are_accessible() {
    let db = make_db();
    let context = CssPackagingContext {
      db,
      project_root: PathBuf::from("/tmp/project"),
      output_dir: PathBuf::from("/tmp/project/dist"),
    };
    assert_eq!(context.project_root, PathBuf::from("/tmp/project"));
    assert_eq!(context.output_dir, PathBuf::from("/tmp/project/dist"));
  }

  // ---------------------------------------------------------------------------
  // Test 1: Single asset — output contains CSS rule, no stray @import
  // ---------------------------------------------------------------------------

  #[test]
  fn single_asset_css_is_included_in_output() {
    let db = make_db();
    db.put("asset_1", b"body { color: red; }").unwrap();

    let asset = make_asset("asset_1");
    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp/project"),
        output_dir: PathBuf::from("/tmp/project/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_1")
      .expect("package() must succeed");
    let output = output_string(&result);

    assert!(
      output.contains("body"),
      "output should contain 'body' selector; got: {output:?}"
    );
    assert!(
      output.contains("color: red"),
      "output should contain 'color: red'; got: {output:?}"
    );
    assert!(
      !output.contains("@import"),
      "output must not contain leftover @import; got: {output:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // Test 2: Multiple assets — source order is preserved in output
  // ---------------------------------------------------------------------------

  #[test]
  fn multiple_assets_are_concatenated_in_source_order() {
    let db = make_db();
    db.put("asset_1", b"h1 { font-size: 2em; }").unwrap();
    db.put("asset_2", b"p { margin: 0; }").unwrap();

    let asset1 = make_asset("asset_1");
    let asset2 = make_asset("asset_2");
    let bundle = make_bundle("bundle_1", vec!["asset_1", "asset_2"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset1, asset2]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp/project"),
        output_dir: PathBuf::from("/tmp/project/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_1")
      .expect("package() must succeed");
    let output = output_string(&result);

    let h1_pos = output.find("h1").expect("output must contain 'h1'");
    let p_pos = output.find('p').expect("output must contain 'p'");
    assert!(
      h1_pos < p_pos,
      "'h1' must appear before 'p' (source order); got: {output:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // Test 3: External @import is hoisted to the top of the output and deduplicated
  // ---------------------------------------------------------------------------

  #[test]
  fn external_import_is_hoisted_and_deduplicated() {
    let db = make_db();
    // asset_1 explicitly imports the external URL.
    // The packager should strip this and hoist it manually.
    let ext_url = "https://fonts.googleapis.com/css?family=Roboto";
    let css_content = format!("@import \"{}\";\nbody {{ color: red; }}", ext_url);
    db.put("asset_1", css_content.as_bytes()).unwrap();

    let asset = make_asset("asset_1");
    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    // Dependency marked as Sync and not skipped -> triggers hoisting logic
    let ext_dep = make_dependency(ext_url, Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_1".to_string(), vec![ext_dep]);
    // No resolved asset -> external

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp/project"),
        output_dir: PathBuf::from("/tmp/project/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_1")
      .expect("package() must succeed");
    let output = output_string(&result);

    let import_stmt = format!("@import \"{}\";", ext_url);

    // Check it appears exactly once
    let matches: Vec<_> = output.matches(&import_stmt).collect();
    assert_eq!(
      matches.len(),
      1,
      "External @import should appear exactly once in output"
    );

    let import_pos = output.find(&import_stmt).unwrap();
    let body_pos = output.find("body").unwrap();
    assert!(import_pos < body_pos, "Hoisted import must be at the top");
  }

  // ---------------------------------------------------------------------------
  // Test 4: Internal @import is resolved, inlined, and deduplicated
  // ---------------------------------------------------------------------------

  #[test]
  fn internal_import_is_inlined_and_deduplicated() {
    let db = make_db();
    // asset_1 imports asset_2.
    // asset_2 has specific content we can track.
    db.put("asset_1", b"@import \"asset_2\";\n.asset1 {}")
      .unwrap();
    db.put("asset_2", b".asset2 {}").unwrap();

    let asset1 = make_asset("asset_1");
    let asset2 = make_asset("asset_2");
    // Bundle contains both. Typically source order might put imported assets first if they are deps.
    let bundle = make_bundle("bundle_1", vec!["asset_1", "asset_2"]);

    let internal_dep = make_dependency("asset_2", Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());

    // Scenario: asset_2 is in the bundle asset list AND imported by asset_1.
    // The entry file will try to import both asset_2 and asset_1.
    // asset_1 will also import asset_2.
    // Result should ideally handle this gracefully (deduplication or harmless redundancy).
    // Note: If deduplication works, .asset2 {} might appear once.
    // If not, it might appear twice (once from entry->asset_2, once from entry->asset_1->asset_2).
    // Lightningcss bundler usually handles this if specifiers match.

    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset2.clone(), asset1]);
    graph
      .deps_by_asset
      .insert("asset_1".to_string(), vec![internal_dep]);
    graph.resolved.insert("asset_2".to_string(), asset2);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp/project"),
        output_dir: PathBuf::from("/tmp/project/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_1").expect("should succeed");
    let output = output_string(&result);

    assert!(output.contains(".asset1"), "Should contain asset1 content");
    assert!(output.contains(".asset2"), "Should contain asset2 content");

    // Verify no leftover @import
    assert!(
      !output.contains("@import \"asset_2\""),
      "Internal import should be compiled away"
    );

    // Optional: Check duplication.
    // lightningcss deduplicates imports based on file path.
    // Since we use the same path ("asset_2") in the map, it should be deduplicated.
    let matches: Vec<_> = output.matches(".asset2").collect();
    assert_eq!(
      matches.len(),
      1,
      "Content of asset_2 should appear exactly once (deduplicated)"
    );
  }

  // ---------------------------------------------------------------------------
  // Test 6: Gap - Bundle ID equals Asset ID
  // ---------------------------------------------------------------------------

  #[test]
  fn handles_bundle_id_colliding_with_asset_id() {
    let db = make_db();
    db.put("foo", b".foo { color: blue; }").unwrap();

    let asset = make_asset("foo");
    // Bundle ID is also "foo"
    let bundle = make_bundle("foo", vec!["foo"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("foo".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("foo").expect("should succeed");
    let output = output_string(&result);

    // FIXED BEHAVIOR: Output should contain the asset content.
    assert!(
      output.contains(".foo"),
      "Asset content should be present even if bundle ID matches asset ID"
    );
  }

  // ---------------------------------------------------------------------------
  // Test 7: Gap - Asset with empty content
  // ---------------------------------------------------------------------------

  #[test]
  fn handles_empty_asset_content() {
    let db = make_db();
    db.put("empty", b"").unwrap();
    db.put("normal", b".normal {}").unwrap();

    let asset_empty = make_asset("empty");
    let asset_normal = make_asset("normal");
    let bundle = make_bundle("bundle_1", vec!["empty", "normal"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset_empty, asset_normal]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_1").expect("should succeed");
    let output = output_string(&result);

    assert!(output.contains(".normal"));
    // Empty content should just result in no extra text, no errors.
  }

  // ---------------------------------------------------------------------------
  // Test 8: Verify limitation - Imported asset NOT in bundle is not resolved
  // ---------------------------------------------------------------------------

  #[test]
  fn internal_import_missing_from_bundle_is_not_resolved() {
    let db = make_db();
    db.put("asset_1", b"@import \"asset_2\";").unwrap();
    db.put("asset_2", b".asset2 {}").unwrap();

    let asset1 = make_asset("asset_1");
    // asset_2 exists in DB but is NOT in the bundle asset list.

    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    // Only asset_1 is in the bundle list.
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset1]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_1").expect("should succeed");
    let output = output_string(&result);

    // Because asset_2 is not in the bundle list, it's not in the InMemoryCssProvider map.
    // So the import resolves to empty string (or remains as an import if not found?
    // InMemoryCssProvider returns empty string for unknown files).
    // So the output should NOT contain .asset2 {}.

    assert!(
      !output.contains(".asset2"),
      "Content of asset_2 should be missing because it is not in the bundle"
    );
    // It is effectively treated as an empty file.
  }
}
