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

use crate::{CssPackager, CssPackagingContext, url_replacer};

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

/// Removes CSS rules whose selector matches an unused CSS Module class.
///
/// Uses the lightningcss AST to correctly handle:
/// - CSS comments (stripped by the parser)
/// - Rules nested inside at-rules (`@media`, `@supports`, `@layer`, `@starting-style`)
/// - Grouped selectors (`.a, .b { }`) — rule removed only if ALL selectors are unused module
///   classes; if any selector is used or not a module selector, the whole rule is kept
///
/// Falls back to returning the original CSS string if parsing or serialization fails.
fn remove_unused_class_rules(
  css: &str,
  all_module_selectors: &std::collections::HashSet<String>,
  used_selectors: &std::collections::HashSet<String>,
) -> String {
  use lightningcss::stylesheet::StyleSheet;

  let mut stylesheet = match StyleSheet::parse(css, Default::default()) {
    Ok(ss) => ss,
    Err(_) => return css.to_string(),
  };

  remove_unused_from_rule_list(&mut stylesheet.rules, all_module_selectors, used_selectors);

  match stylesheet.to_css(Default::default()) {
    Ok(result) => result.code,
    Err(_) => css.to_string(),
  }
}

/// Recursively removes unused CSS Module class rules from a rule list.
/// Handles `@media`, `@supports`, `@layer`, `@starting-style`, `@container`, and `@scope` nesting.
fn remove_unused_from_rule_list<'i>(
  rule_list: &mut lightningcss::rules::CssRuleList<'i>,
  all_module_selectors: &std::collections::HashSet<String>,
  used_selectors: &std::collections::HashSet<String>,
) {
  use lightningcss::rules::CssRule;
  use lightningcss::traits::ToCss;

  rule_list.0.retain_mut(|rule| match rule {
    CssRule::Style(style_rule) => {
      // A rule is removed only when ALL its selectors are unused module classes.
      // If any selector is used or is not a known module selector, retain the rule.
      let all_unused = style_rule.selectors.0.iter().all(|selector| {
        let selector_str = selector
          .to_css_string(Default::default())
          .unwrap_or_default();
        all_module_selectors.contains(&selector_str) && !used_selectors.contains(&selector_str)
      });
      !all_unused
    }
    CssRule::Media(media_rule) => {
      remove_unused_from_rule_list(&mut media_rule.rules, all_module_selectors, used_selectors);
      true
    }
    CssRule::Supports(supports_rule) => {
      remove_unused_from_rule_list(
        &mut supports_rule.rules,
        all_module_selectors,
        used_selectors,
      );
      true
    }
    CssRule::LayerBlock(layer_rule) => {
      remove_unused_from_rule_list(&mut layer_rule.rules, all_module_selectors, used_selectors);
      true
    }
    CssRule::StartingStyle(starting_style_rule) => {
      remove_unused_from_rule_list(
        &mut starting_style_rule.rules,
        all_module_selectors,
        used_selectors,
      );
      true
    }
    CssRule::Container(container_rule) => {
      remove_unused_from_rule_list(
        &mut container_rule.rules,
        all_module_selectors,
        used_selectors,
      );
      true
    }
    CssRule::Scope(scope_rule) => {
      remove_unused_from_rule_list(&mut scope_rule.rules, all_module_selectors, used_selectors);
      true
    }
    _ => true,
  });
}

/// Applies CSS Module tree-shaking to a single asset's CSS string.
/// Returns the CSS with unused class rules removed.
/// Returns the CSS unchanged if the asset is not a CSS Module, has no symbol info,
/// or optimization is disabled for the asset's environment.
pub(crate) fn apply_css_module_tree_shaking(
  css: &str,
  asset: &atlaspack_core::types::Asset,
  used_symbols: &std::collections::HashSet<String>,
) -> String {
  use std::collections::HashSet;

  // Only apply in production (optimized) builds
  if !asset.env.should_optimize {
    return css.to_string();
  }

  // Only process CSS Module assets (those with a non-empty symbols list)
  let symbols = match asset.symbols.as_ref() {
    Some(s) if !s.is_empty() => s,
    _ => return css.to_string(),
  };

  // Build exported->local mapping from asset symbols
  let symbol_map: std::collections::HashMap<String, String> = symbols
    .iter()
    .map(|s| (s.exported.clone(), s.local.clone()))
    .collect();

  // All CSS selector names from this module (e.g. ".foo_abc123")
  let all_module_selectors: HashSet<String> =
    symbols.iter().map(|s| format!(".{}", s.local)).collect();

  // Selectors that are actually used (mapped from exported names to local names)
  let used_selectors: HashSet<String> = used_symbols
    .iter()
    .filter_map(|exported| symbol_map.get(exported))
    .map(|local| format!(".{}", local))
    .collect();

  remove_unused_class_rules(css, &all_module_selectors, &used_selectors)
}

/// Applies CSS Module tree-shaking directly on a lightningcss `CssRuleList` (post-bundling AST).
/// This is the correct integration point: called after lightningcss has resolved all @imports so
/// the selector set is stable. Unlike `apply_css_module_tree_shaking` (which re-parses a string),
/// this mutates the already-parsed AST in-place, avoiding a redundant parse/print cycle.
fn apply_css_module_tree_shaking_ast<'i>(
  rules: &mut lightningcss::rules::CssRuleList<'i>,
  asset: &atlaspack_core::types::Asset,
  used_symbols: &std::collections::HashSet<String>,
) {
  use std::collections::HashSet;

  // Only process CSS Module assets (those with a non-empty symbols list).
  let symbols = match asset.symbols.as_ref() {
    Some(s) if !s.is_empty() => s,
    _ => return,
  };

  // Build exported→local mapping from asset symbols.
  let symbol_map: std::collections::HashMap<String, String> = symbols
    .iter()
    .map(|s| (s.exported.clone(), s.local.clone()))
    .collect();

  // All local CSS selector names for this module (e.g. ".foo_abc123").
  let all_module_selectors: HashSet<String> =
    symbols.iter().map(|s| format!(".{}", s.local)).collect();

  // Selectors that are actually used (mapped from exported names to local names).
  let used_selectors: HashSet<String> = used_symbols
    .iter()
    .filter_map(|exported| symbol_map.get(exported))
    .map(|local| format!(".{}", local))
    .collect();

  remove_unused_from_rule_list(rules, &all_module_selectors, &used_selectors);
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
    let mut entry_contents = String::new();
    let mut css_code_map: HashMap<String, String> = HashMap::new();

    for asset in &assets {
      // Emit a synthetic @import for this asset in the bundle entry.
      entry_contents.push_str(&format!("@import \"{}\";\n", asset.id));

      // Identify unresolvable Sync deps — these are external @imports to hoist.
      // Collect external specifiers per-asset so stripping is scoped to only that
      // asset's own imports (avoids cross-contaminating other assets).
      let deps = self.bundle_graph.get_dependencies(asset)?;
      let mut asset_external_specifiers: Vec<String> = Vec::new();
      for dep in deps {
        if dep.priority != Priority::Sync {
          continue;
        }
        if self.bundle_graph.is_dependency_skipped(dep) {
          continue;
        }
        let resolved = self.bundle_graph.get_resolved_asset(dep, bundle)?;
        if resolved.is_none() {
          // Deduplicate hoisted imports globally.
          let import_stmt = format!("@import \"{}\";", dep.specifier);
          if !hoisted_imports.contains(&import_stmt) {
            hoisted_imports.push(import_stmt);
          }
          asset_external_specifiers.push(dep.specifier.clone());
        }
      }

      // Read CSS content from the database (fall back to asset.id as the key).
      let db_key = asset.content_key.as_deref().unwrap_or(&asset.id);
      let css_bytes = self.context.db.get(db_key)?.unwrap_or_default();
      let css_code = String::from_utf8(css_bytes)
        .map_err(|e| anyhow::anyhow!("Asset {} CSS is not valid UTF-8: {e}", asset.id))?;

      // Strip external @import lines from this asset's CSS before handing to the Bundler.
      // This prevents the Bundler from encountering unresolvable URLs, which could error
      // or produce duplicate imports alongside our manually hoisted ones.
      let filtered_css = if asset_external_specifiers.is_empty() {
        css_code
      } else {
        filter_external_imports(&css_code, &asset_external_specifiers)
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

    // Phase 4: CSS Module tree-shaking (production-only).
    // Runs on the post-bundling AST so lightningcss has already resolved all @imports
    // and the selector set is stable before pruning.
    let mut stylesheet = stylesheet;
    let mut warnings: Vec<atlaspack_core::types::Diagnostic> = Vec::new();
    if bundle.env.should_optimize {
      for asset in &assets {
        if let Some(used_syms) = self.bundle_graph.get_used_symbols(&asset.id) {
          if used_syms.contains("*") {
            // Wildcard import: retain all classes.
            continue;
          }
          // Check for default import guard.
          let has_default_import = used_syms.contains("default") && {
            self
              .bundle_graph
              .get_incoming_dependencies(asset)
              .ok()
              .map(|deps| {
                deps.iter().any(|dep| {
                  dep
                    .symbols
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .any(|s| s.exported == "default")
                })
              })
              .unwrap_or(false)
          };
          if has_default_import {
            warnings.push(
              atlaspack_core::types::DiagnosticBuilder::default()
                .message(format!(
                  "CSS modules cannot be tree shaken when imported with a default specifier ({})",
                  asset.file_path.display()
                ))
                .hints(vec![
                  "Instead use: import * as styles from \"...\";".to_string(),
                ])
                .origin(Some("atlaspack_packager_css".to_string()))
                .build()
                .unwrap(),
            );
            continue;
          }
          // Apply tree-shaking to the bundled AST using the asset's symbol table.
          apply_css_module_tree_shaking_ast(&mut stylesheet.rules, asset, &used_syms);
        }
      }
    }

    let result = stylesheet
      .to_css(PrinterOptions::default())
      .map_err(|e| anyhow::anyhow!("lightningcss printing failed: {:?}", e))?;
    let mut css = result.code;

    // Phase 5: prepend hoisted external @imports before all inlined rules.
    if !hoisted_imports.is_empty() {
      let hoisted = hoisted_imports.join("\n");
      css = format!("{hoisted}\n{css}");
    }

    // Phase 6: Replace URL reference placeholders with resolved paths or data URIs.
    let output_dir = &self.context.output_dir;
    let css = url_replacer::replace_url_references(
      &css,
      bundle,
      self.bundle_graph.as_ref(),
      &self.context.db,
      output_dir,
    )?;

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
      warnings,
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
    /// asset_id → set of used exported symbol names (for CSS Modules tree-shaking).
    used_symbols_by_asset: HashMap<String, HashSet<String>>,
    /// asset_id → incoming dependencies (for testing default import guard).
    incoming_deps_by_asset: HashMap<String, Vec<Dependency>>,
  }

  impl TestBundleGraph {
    fn new() -> Self {
      Self {
        bundles: Vec::new(),
        assets_by_bundle: HashMap::new(),
        deps_by_asset: HashMap::new(),
        resolved: HashMap::new(),
        skipped: HashSet::new(),
        used_symbols_by_asset: HashMap::new(),
        incoming_deps_by_asset: HashMap::new(),
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

    fn get_incoming_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
      Ok(
        self
          .incoming_deps_by_asset
          .get(&asset.id)
          .map(|v| v.iter().collect())
          .unwrap_or_default(),
      )
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

    fn get_used_symbols(&self, asset_id: &str) -> Option<HashSet<String>> {
      self.used_symbols_by_asset.get(asset_id).cloned()
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
  // CSS Modules tree-shaking tests
  // These tests reference functions that do not exist yet — compile errors are expected.
  // ---------------------------------------------------------------------------

  // Helper: build an Asset with the given symbols list (exported name → local mangled name).
  // Each tuple is (exported, local) matching Symbol::exported and Symbol::local.
  fn make_css_module_asset(id: &str, symbols: Vec<(&str, &str)>, should_optimize: bool) -> Asset {
    use atlaspack_core::types::Symbol;
    Asset {
      id: id.to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment {
        should_optimize,
        ..Environment::default()
      }),
      symbols: if symbols.is_empty() {
        None
      } else {
        Some(
          symbols
            .into_iter()
            .map(|(exported, local)| Symbol {
              exported: exported.to_string(),
              local: local.to_string(),
              loc: None,
              is_weak: false,
              is_esm_export: false,
              self_referenced: false,
              is_static_binding_safe: true,
            })
            .collect(),
        )
      },
      ..Asset::default()
    }
  }

  // --- Test A: unused class is removed ---
  #[test]
  fn tree_shaking_removes_unused_class() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    let used: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      "used class .foo_abc must be retained; got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      "unused class .bar_def must be removed; got: {output:?}"
    );
  }

  // --- Test B: used class is retained ---
  #[test]
  fn tree_shaking_retains_used_class() {
    let css = ".foo_abc { color: red; }";
    let all: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      "used class .foo_abc must appear in output; got: {output:?}"
    );
  }

  // --- Test C: non-module selector is not touched even when all module classes are unused ---
  #[test]
  fn tree_shaking_preserves_non_module_selectors() {
    let css = ".foo_abc { color: red; } body { margin: 0; }";
    let all: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = HashSet::new(); // foo_abc unused

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".foo_abc"),
      "unused module class .foo_abc must be removed; got: {output:?}"
    );
    assert!(
      output.contains("body"),
      "non-module selector body must be retained; got: {output:?}"
    );
    assert!(
      output.contains("margin"),
      "body rule body must be retained; got: {output:?}"
    );
  }

  // --- Test D: when used_selectors == all_module_selectors, everything is retained (wildcard) ---
  #[test]
  fn tree_shaking_wildcard_retains_all_classes() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    // wildcard: used == all
    let used = all.clone();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be retained under wildcard; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be retained under wildcard; got: {output:?}"
    );
  }

  // --- Test E: empty used set removes all module classes ---
  #[test]
  fn tree_shaking_empty_used_symbols_removes_all_module_classes() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    let used: HashSet<String> = HashSet::new();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".foo_abc"),
      ".foo_abc must be removed; got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      ".bar_def must be removed; got: {output:?}"
    );
  }

  // --- Test F: multi-line rule is fully removed ---
  #[test]
  fn tree_shaking_removes_multiline_unused_rule() {
    let css = ".unused_xyz {\n  color: blue;\n  font-size: 12px;\n}";
    let all: HashSet<String> = [".unused_xyz"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = HashSet::new();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".unused_xyz"),
      "selector must be removed; got: {output:?}"
    );
    assert!(
      !output.contains("font-size: 12px"),
      "rule body must also be removed; got: {output:?}"
    );
  }

  // --- Test G: multiple rules, partial removal ---
  #[test]
  fn tree_shaking_partial_removal_keeps_used_removes_unused() {
    let css = ".a_111 { color: red; } .b_222 { color: blue; } .c_333 { color: green; }";
    let all: HashSet<String> = [".a_111", ".b_222", ".c_333"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    let used: HashSet<String> = [".a_111", ".c_333"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".a_111"),
      ".a_111 must be retained; got: {output:?}"
    );
    assert!(
      output.contains(".c_333"),
      ".c_333 must be retained; got: {output:?}"
    );
    assert!(
      !output.contains(".b_222"),
      ".b_222 must be removed; got: {output:?}"
    );
  }

  // --- Test H: dev mode (should_optimize = false) — apply_css_module_tree_shaking is a no-op ---
  #[test]
  fn tree_shaking_is_skipped_in_dev_mode() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    // Asset has symbols but should_optimize = false
    let asset = make_css_module_asset(
      "asset_dev",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      false, // dev mode
    );
    let used: HashSet<String> = ["foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    // In dev mode the function must return css unchanged
    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present in dev mode output; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be present in dev mode output (no tree-shaking); got: {output:?}"
    );
  }

  // --- Test K: Production mode — unused symbols ARE removed end-to-end ---
  #[test]
  fn tree_shaking_is_applied_in_production_mode() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    // Asset has symbols and should_optimize = true
    let asset = make_css_module_asset(
      "asset_prod",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true, // production mode
    );
    // Only "foo" is used (mapping to "foo_abc")
    let used: HashSet<String> = ["foo"].iter().map(|s| s.to_string()).collect();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present in prod mode output (used); got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      ".bar_def must be REMOVED in prod mode output (unused); got: {output:?}"
    );
  }

  // --- Test L: Wildcard import in package() protects against tree shaking ---
  #[test]
  fn wildcard_import_disables_tree_shaking_in_package() {
    let db = make_db();
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    db.put("asset_wildcard", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_wildcard",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true, // production
    );

    let bundle = make_bundle("bundle_w", vec!["asset_wildcard"]);
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_w".to_string(), vec![asset]);

    // Used symbols contains "*", implying wildcard import
    let mut used_syms = HashSet::new();
    used_syms.insert("*".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_wildcard".to_string(), used_syms);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_w").expect("should succeed");
    let output = output_string(&result);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present (wildcard); got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be present (wildcard); got: {output:?}"
    );
  }

  // --- Test M: Default import guard ---
  #[test]
  fn default_import_disables_tree_shaking() {
    use atlaspack_core::types::Symbol;

    let db = make_db();
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    db.put("asset_default", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_default",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true, // production
    );

    let bundle = make_bundle("bundle_d", vec!["asset_default"]);
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_d".to_string(), vec![asset.clone()]);

    // Used symbols contains "default"
    let mut used_syms = HashSet::new();
    used_syms.insert("default".to_string());
    // Also mark 'foo' as used, but 'bar' unused.
    // If guard works, 'bar' will still be kept.
    used_syms.insert("foo".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_default".to_string(), used_syms);

    // Create incoming dependency that imports "default"
    let mut dep = make_dependency("asset_default", Priority::Sync);
    dep.symbols = Some(vec![Symbol {
      exported: "default".to_string(),
      local: "default".to_string(),
      ..Symbol::default()
    }]);
    graph
      .incoming_deps_by_asset
      .insert("asset_default".to_string(), vec![dep]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_d").expect("should succeed");
    let output = output_string(&result);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be present (default import guard); got: {output:?}"
    );
  }

  // --- Test I: plain CSS asset with no symbols is not modified ---
  #[test]
  fn tree_shaking_no_op_for_asset_without_symbols() {
    let css = ".plain { color: red; }";
    // Asset with symbols = None (plain CSS, not a CSS Module)
    let asset = Asset {
      id: "asset_plain".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment {
        should_optimize: true,
        ..Environment::default()
      }),
      symbols: None,
      ..Asset::default()
    };
    let used: HashSet<String> = HashSet::new();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    assert_eq!(
      output, css,
      "plain CSS asset with no symbols must be returned unchanged"
    );
  }

  // --- Test J: asset with empty symbols vec is not modified ---
  #[test]
  fn tree_shaking_no_op_for_asset_with_empty_symbols() {
    let css = ".plain { color: red; }";
    let asset = make_css_module_asset("asset_empty_syms", vec![], true);
    // make_css_module_asset returns symbols = None when vec is empty,
    // which is the correct representation for "no CSS module exports"
    let used: HashSet<String> = HashSet::new();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    assert_eq!(
      output, css,
      "asset with empty symbols must be returned unchanged"
    );
  }

  // --- Test N: composes retention — composed-from class is retained when composing class is used ---
  //
  // When class `foo` composes from class `bar` (local composes), the CSS transformer
  // emits both `foo` and `bar` as symbols. If `foo` is in `used_symbols`, `bar` must
  // also appear there (the JS runtime value of `foo` includes `bar`'s local name as a
  // space-separated string). This test verifies that when both `foo` and `bar` are in
  // `used_symbols`, both their CSS rules are retained — i.e. the tree-shaker does not
  // incorrectly prune a composed-from class that appears in `used_symbols`.
  #[test]
  fn tree_shaking_retains_composed_from_class_when_in_used_symbols() {
    // .foo_abc composes from .bar_def (local composes).
    // Both are exported as symbols by the CSS transformer.
    let css = ".foo_abc { color: red; } .bar_def { font-weight: bold; }";
    let asset = make_css_module_asset(
      "asset_composes",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true, // production mode
    );
    // foo is directly used; bar appears in used_symbols because foo composes from bar
    // (the JS bundleGraph propagates bar as used when foo is used).
    let used: HashSet<String> = ["foo", "bar"].iter().map(|s| s.to_string()).collect();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc (composing class) must be retained; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def (composed-from class) must be retained when in used_symbols; got: {output:?}"
    );
  }

  // --- Test O: @media block is retained even when its only rule is removed ---
  #[test]
  fn tree_shaking_retains_empty_media_block_after_removing_nested_rule() {
    // When the only rule inside @media is unused, the rule is removed.
    // The @media block itself may remain (empty) or be omitted — both are acceptable.
    // We just verify there is no crash and the unused class is absent.
    let css = "@media (min-width: 500px) { .unused_xyz { color: red; } }";
    let all: HashSet<String> = [".unused_xyz"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = HashSet::new();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".unused_xyz"),
      "unused class inside @media must be removed by AST shaker; got: {output:?}"
    );
  }

  // --- Test P: CSS comment containing a brace does not corrupt the output ---
  //
  // The current brace-depth scanner does not handle comments, so `/* } */` will
  // confuse its depth counter and corrupt the output. The AST-based replacement
  // must handle this correctly because lightningcss parses comments before the
  // tree-shaker sees the stylesheet.
  #[test]
  fn tree_shaking_comment_with_brace_does_not_corrupt_output() {
    let css = "/* } */ .foo_abc { color: red; } .bar_def { color: blue; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    let used: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      "used class .foo_abc must be retained; got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      "unused class .bar_def must be removed; got: {output:?}"
    );
    // Verify the output is not corrupted: it must not contain mismatched braces.
    // A simple proxy: the number of '{' and '}' in the output must be equal.
    let open_braces = output.chars().filter(|&c| c == '{').count();
    let close_braces = output.chars().filter(|&c| c == '}').count();
    assert_eq!(
      open_braces, close_braces,
      "output must have balanced braces (no corruption from comment brace); got: {output:?}"
    );
  }

  // --- Test Q: class rule inside @media is tree-shaken when unused ---
  //
  // The AST-based implementation must descend into at-rule blocks and remove
  // unused CSS Module class rules nested inside them. The brace scanner cannot
  // do this (known limitation documented in Test O).
  #[test]
  fn tree_shaking_removes_unused_class_inside_media_query() {
    let css = "@media (min-width: 500px) { .unused_xyz { color: red; } }";
    let all: HashSet<String> = [".unused_xyz"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = HashSet::new();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".unused_xyz"),
      "unused class inside @media must be removed by AST-based shaker; got: {output:?}"
    );
    // The @media block may be empty or absent — either is acceptable.
  }

  // --- Test R: class rule inside @media is retained when used ---
  //
  // When the nested class IS in `used_selectors`, the AST-based implementation
  // must keep the rule (and the surrounding @media block).
  #[test]
  fn tree_shaking_retains_used_class_inside_media_query() {
    let css = "@media (min-width: 500px) { .used_abc { color: red; } }";
    let all: HashSet<String> = [".used_abc"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = [".used_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".used_abc"),
      "used class inside @media must be retained; got: {output:?}"
    );
    assert!(
      output.contains("@media"),
      "@media block must be retained when its rule is used; got: {output:?}"
    );
  }

  // --- Test S: grouped selector — unused group is removed ---
  //
  // When both selectors in a comma-separated group are in `all_module_selectors`
  // but neither is in `used_selectors`, the entire rule must be removed.
  #[test]
  fn tree_shaking_removes_fully_unused_grouped_selector() {
    let css = ".foo_abc, .bar_def { color: red; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    let used: HashSet<String> = HashSet::new();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".foo_abc"),
      ".foo_abc must be absent when no selectors in the group are used; got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      ".bar_def must be absent when no selectors in the group are used; got: {output:?}"
    );
  }

  // --- Test T: grouped selector — partially used group retains the rule ---
  //
  // When at least one selector in a comma-separated group is used, the safe
  // behaviour is to retain the entire rule. The AST-based implementation may
  // alternatively strip only the unused selectors from the group, but it must
  // not drop the used one.
  #[test]
  fn tree_shaking_retains_grouped_selector_rule_when_any_selector_is_used() {
    let css = ".foo_abc, .bar_def { color: red; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    // Only foo is used; bar is not.
    let used: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present because it is used; got: {output:?}"
    );
    // The rule body must be retained (it applies to the used selector).
    assert!(
      output.contains("color: red"),
      "rule body must be retained when a selector in the group is used; got: {output:?}"
    );
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

  // ---------------------------------------------------------------------------
  // Test U: @container — unused class nested inside @container is removed
  // ---------------------------------------------------------------------------

  #[test]
  fn tree_shaking_removes_unused_class_inside_container_query() {
    let css = "@container sidebar (min-width: 700px) { .foo_abc { color: red; } .bar_def { color: blue; } }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    // Only foo is used.
    let used: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be retained inside @container; got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      ".bar_def must be removed inside @container; got: {output:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // Test V: @scope — unused class nested inside @scope is removed
  // ---------------------------------------------------------------------------

  #[test]
  fn tree_shaking_removes_unused_class_inside_scope_rule() {
    let css = "@scope (.card) { .foo_abc { color: red; } .bar_def { color: blue; } }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    // Only foo is used.
    let used: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be retained inside @scope; got: {output:?}"
    );
    assert!(
      !output.contains(".bar_def"),
      ".bar_def must be removed inside @scope; got: {output:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // Test W: per-asset external specifier isolation — asset_2's @import is NOT
  // stripped from asset_1's CSS even if asset_2 has its own external import
  // ---------------------------------------------------------------------------

  #[test]
  fn external_specifier_stripping_is_scoped_to_per_asset() {
    let db = make_db();

    // asset_1 has a regular rule — no external imports.
    db.put("asset_1", b".a { color: red; }").unwrap();
    // asset_2 has an external @import AND its own rule.
    let ext_url = "https://fonts.googleapis.com/css?family=Roboto";
    db.put(
      "asset_2",
      format!("@import \"{ext_url}\";\n.b {{ color: blue; }}").as_bytes(),
    )
    .unwrap();

    let asset1 = make_asset("asset_1");
    let asset2 = make_asset("asset_2");
    let bundle = make_bundle("bundle_1", vec!["asset_1", "asset_2"]);

    // Only asset_2 has the external dep.
    let ext_dep = make_dependency(ext_url, Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset1, asset2]);
    // asset_2 has the external dep; asset_1 has none.
    graph
      .deps_by_asset
      .insert("asset_2".to_string(), vec![ext_dep]);
    // No resolved asset for ext_url -> external.

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

    // Both asset rules must appear.
    assert!(
      output.contains(".a"),
      "asset_1 rule must be present; got: {output:?}"
    );
    assert!(
      output.contains(".b"),
      "asset_2 rule must be present; got: {output:?}"
    );

    // The external import is hoisted once.
    let import_stmt = format!("@import \"{ext_url}\";");
    let occurrences: Vec<_> = output.matches(&import_stmt).collect();
    assert_eq!(
      occurrences.len(),
      1,
      "External @import must appear exactly once; got: {output:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // Test X: default import warning is captured in PackageResult.warnings,
  // not printed to stderr, and all classes are retained.
  // ---------------------------------------------------------------------------

  #[test]
  fn default_import_emits_structured_warning_in_package_result() {
    use atlaspack_core::types::Symbol;

    let db = make_db();
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    db.put("asset_default", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_default",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true, // production
    );

    let mut bundle = make_bundle("bundle_d2", vec!["asset_default"]);
    // Use a production environment so the tree-shaking phase (gated on bundle.env.should_optimize)
    // is active and the default-import guard can fire.
    bundle.env.should_optimize = true;
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_d2".to_string(), vec![asset.clone()]);

    // used_symbols contains "default", triggering the guard.
    let mut used_syms = HashSet::new();
    used_syms.insert("default".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_default".to_string(), used_syms);

    // Incoming dependency that exports "default".
    let mut dep = make_dependency("asset_default", Priority::Sync);
    dep.symbols = Some(vec![Symbol {
      exported: "default".to_string(),
      local: "default".to_string(),
      loc: None,
      is_weak: false,
      is_esm_export: false,
      self_referenced: false,
      is_static_binding_safe: true,
    }]);
    graph
      .incoming_deps_by_asset
      .insert("asset_default".to_string(), vec![dep]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_d2").expect("should succeed");
    let output = output_string(&result);

    // Both classes are retained because tree-shaking was skipped.
    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be retained when default import guard fires; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be retained when default import guard fires; got: {output:?}"
    );

    // A structured warning must have been emitted into PackageResult.warnings.
    assert!(
      !result.warnings.is_empty(),
      "PackageResult.warnings must be non-empty when default import guard fires"
    );
    let warning_msg = &result.warnings[0].message;
    assert!(
      warning_msg.contains("default specifier"),
      "Warning message must mention 'default specifier'; got: {warning_msg:?}"
    );
  }
}
