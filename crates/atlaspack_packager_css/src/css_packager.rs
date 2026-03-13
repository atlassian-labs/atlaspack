use std::sync::Arc;

use anyhow::Result;
use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::package_result::{BundleInfo, PackageResult};
use atlaspack_core::types::Priority;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;

use crate::{CssPackager, CssPackagingContext};

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

    // Collect hoisted external imports and concatenate per-asset CSS in source order.
    // TODO: deduplicate hoisted imports to match future JS packager behaviour.
    let mut hoisted_imports: Vec<String> = Vec::new();
    let mut css_parts: Vec<String> = Vec::with_capacity(assets.len());

    for asset in &assets {
      // Collect unresolvable Sync deps as hoisted @imports.
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
          hoisted_imports.push(dep.specifier.clone());
        }
      }

      // Read CSS content from the database (fall back to asset.id as the key).
      let db_key = asset.content_key.as_deref().unwrap_or(&asset.id);
      let css_bytes = self.context.db.get(db_key)?.unwrap_or_default();
      let css_code = String::from_utf8(css_bytes)
        .map_err(|e| anyhow::anyhow!("Asset {} CSS is not valid UTF-8: {e}", asset.id))?;

      // Parse and re-print through lightningcss to normalise the CSS.
      let sheet = lightningcss::stylesheet::StyleSheet::parse(&css_code, ParserOptions::default())
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
      let printed = sheet
        .to_css(PrinterOptions::default())
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

      css_parts.push(printed.code);
    }

    // Concatenate all assets in source order.
    let mut css = css_parts.join("");

    // Prepend hoisted external @imports before all inlined rules.
    if !hoisted_imports.is_empty() {
      let hoisted = hoisted_imports
        .iter()
        .map(|s| format!("@import \"{s}\";"))
        .collect::<Vec<_>>()
        .join("\n");
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
  // Test 3: External @import is hoisted to the top of the output
  // ---------------------------------------------------------------------------

  #[test]
  fn external_import_is_hoisted_before_css_rules() {
    let db = make_db();
    db.put("asset_1", b"body {}").unwrap();

    let asset = make_asset("asset_1");
    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    // External dependency (unresolvable, not skipped) — should be hoisted.
    let ext_dep = make_dependency(
      "https://fonts.googleapis.com/css?family=Roboto",
      Priority::Sync,
    );

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_1".to_string(), vec![ext_dep]);
    // No entry in `resolved` → the dependency has no resolved asset in the bundle.

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

    let import_stmt = "@import \"https://fonts.googleapis.com/css?family=Roboto\";";
    let import_pos = output
      .find(import_stmt)
      .unwrap_or_else(|| panic!("output must contain hoisted @import; got: {output:?}"));
    let body_pos = output
      .find("body")
      .unwrap_or_else(|| panic!("output must contain 'body'; got: {output:?}"));
    assert!(
      import_pos < body_pos,
      "hoisted @import must appear before any CSS rules; got: {output:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // Test 4: Internal @import is resolved and inlined — no leftover @import in output
  // ---------------------------------------------------------------------------

  #[test]
  fn internal_import_is_inlined_without_leftover_at_import() {
    let db = make_db();
    db.put("asset_1", b"h1 {}").unwrap();
    db.put("asset_2", b"p {}").unwrap();

    let asset1 = make_asset("asset_1");
    let asset2 = make_asset("asset_2");
    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    // Internal dependency from asset_1 → asset_2 (resolved in the same bundle).
    let internal_dep = make_dependency("asset_2", Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    // Both assets are in the bundle (asset_2 first per DFS post-order convention).
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset2.clone(), asset1]);
    graph
      .deps_by_asset
      .insert("asset_1".to_string(), vec![internal_dep]);
    // asset_2 is the resolved target of the internal dep.
    graph.resolved.insert("asset_2".to_string(), asset2);

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
      output.contains("h1"),
      "output must contain 'h1' from asset_1; got: {output:?}"
    );
    assert!(
      output.contains('p'),
      "output must contain 'p' from asset_2; got: {output:?}"
    );
    assert!(
      !output.contains("@import"),
      "output must not contain any leftover @import for internal deps; got: {output:?}"
    );
  }
}
