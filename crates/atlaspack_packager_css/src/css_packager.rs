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

/// In-memory [`SourceProvider`] for lightningcss. Stores CSS by path key so
/// `read<'a>` can return `&'a str` without unsafe code.
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
    // Unknown files (e.g. external URLs) are treated as empty.
    Ok(
      self
        .index
        .get(file)
        .map(|&idx| self.strings[idx].as_str())
        .unwrap_or(""),
    )
  }

  fn resolve(&self, specifier: &str, _originating_file: &Path) -> Result<PathBuf, Self::Error> {
    Ok(PathBuf::from(specifier))
  }
}

#[cfg(test)]
/// Removes rules for unused CSS Module classes from a CSS string via the lightningcss AST.
/// A grouped selector rule is only removed if ALL selectors in the group are unused module
/// classes. Falls back to the original string on parse/serialization failure.
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

fn remove_unused_from_rule_list<'i>(
  rule_list: &mut lightningcss::rules::CssRuleList<'i>,
  all_module_selectors: &std::collections::HashSet<String>,
  used_selectors: &std::collections::HashSet<String>,
) {
  use lightningcss::rules::CssRule;
  use lightningcss::traits::ToCss;

  rule_list.0.retain_mut(|rule| match rule {
    CssRule::Style(style_rule) => {
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

#[cfg(test)]
/// Applies CSS Module tree-shaking to a CSS string. No-op for non-module assets
/// (no symbols), dev builds, or assets with no symbols.
pub(crate) fn apply_css_module_tree_shaking(
  css: &str,
  asset: &atlaspack_core::types::Asset,
  used_symbols: &std::collections::HashSet<String>,
) -> String {
  use std::collections::HashSet;

  if !asset.env.should_optimize {
    return css.to_string();
  }

  // CSS Modules are identified by a non-empty symbols list (no dedicated is_module field).
  let symbols = match asset.symbols.as_ref() {
    Some(s) if !s.is_empty() => s,
    _ => return css.to_string(),
  };

  let symbol_map: std::collections::HashMap<String, String> = symbols
    .iter()
    .map(|s| (s.exported.clone(), s.local.clone()))
    .collect();

  let all_module_selectors: HashSet<String> =
    symbols.iter().map(|s| format!(".{}", s.local)).collect();

  let used_selectors: HashSet<String> = used_symbols
    .iter()
    .filter_map(|exported| symbol_map.get(exported))
    .map(|local| format!(".{}", local))
    .collect();

  remove_unused_class_rules(css, &all_module_selectors, &used_selectors)
}

/// Applies CSS Module tree-shaking directly on the post-bundling AST in-place,
/// avoiding a redundant parse/print cycle.
fn apply_css_module_tree_shaking_ast<'i>(
  rules: &mut lightningcss::rules::CssRuleList<'i>,
  asset: &atlaspack_core::types::Asset,
  used_symbols: &std::collections::HashSet<String>,
) {
  use std::collections::HashSet;

  let symbols = match asset.symbols.as_ref() {
    Some(s) if !s.is_empty() => s,
    _ => return,
  };

  let symbol_map: std::collections::HashMap<String, String> = symbols
    .iter()
    .map(|s| (s.exported.clone(), s.local.clone()))
    .collect();

  let all_module_selectors: HashSet<String> =
    symbols.iter().map(|s| format!(".{}", s.local)).collect();

  let mut used_selectors: HashSet<String> = used_symbols
    .iter()
    .filter_map(|exported| symbol_map.get(exported))
    .map(|local| format!(".{}", local))
    .collect();

  // Expand used_selectors to include classes composed-from by already-used classes.
  // The bundle graph may not always propagate composed-from symbols, so we fall back
  // to parsing `composes:` declarations from the AST.
  expand_composes_selectors(rules, &all_module_selectors, &mut used_selectors);

  remove_unused_from_rule_list(rules, &all_module_selectors, &used_selectors);
}

/// Expands `used_selectors` to a fixed point by following `composes:` declarations in the AST.
fn expand_composes_selectors<'i>(
  rules: &lightningcss::rules::CssRuleList<'i>,
  all_module_selectors: &std::collections::HashSet<String>,
  used_selectors: &mut std::collections::HashSet<String>,
) {
  use lightningcss::properties::Property;
  use lightningcss::rules::CssRule;
  use lightningcss::traits::ToCss;

  loop {
    let mut added_any = false;

    for rule in &rules.0 {
      let style_rule = match rule {
        CssRule::Style(s) => s,
        _ => continue,
      };

      let rule_is_used = style_rule.selectors.0.iter().any(|sel| {
        let s = sel.to_css_string(Default::default()).unwrap_or_default();
        used_selectors.contains(&s)
      });

      if !rule_is_used {
        continue;
      }

      // lightningcss parses `composes: foo;` as `Property::Unparsed` (known property name,
      // value not further typed in non-CSS-modules mode) rather than `Property::Custom`.
      for decl in &style_rule.declarations.declarations {
        use lightningcss::properties::PropertyId;
        use lightningcss::properties::custom::{Token, TokenOrValue};
        let raw_value: Option<String> = match decl {
          Property::Unparsed(unparsed) if unparsed.property_id == PropertyId::Composes => {
            let s = unparsed
              .value
              .0
              .iter()
              .filter_map(|tok| match tok {
                TokenOrValue::Token(Token::Ident(ident)) => Some(ident.as_ref().to_string()),
                _ => None,
              })
              .collect::<Vec<_>>()
              .join(" ");
            Some(s)
          }
          Property::Composes(composes) => {
            let names = composes
              .names
              .iter()
              .map(|n| n.as_ref().to_string())
              .collect::<Vec<_>>()
              .join(" ");
            Some(names)
          }
          _ => None,
        };

        let Some(value) = raw_value else {
          continue;
        };

        // Parse: `<name>+ [from "<file>"|global]`
        // Take all tokens before the first `from` keyword.
        let names_part = value
          .split_whitespace()
          .take_while(|tok| !tok.eq_ignore_ascii_case("from"))
          .collect::<Vec<_>>();

        for name in names_part {
          let selector = format!(".{}", name);
          if all_module_selectors.contains(&selector) && !used_selectors.contains(&selector) {
            used_selectors.insert(selector);
            added_any = true;
          }
        }
      }
    }

    if !added_any {
      break;
    }
  }
}

impl<B: BundleGraph + Send + Sync> CssPackager<B> {
  pub fn new(context: CssPackagingContext, bundle_graph: Arc<B>) -> Self {
    Self {
      context,
      bundle_graph,
    }
  }

  pub fn package(&self, bundle_id: &str) -> Result<PackageResult> {
    let bundle = self
      .bundle_graph
      .get_bundle_by_id(bundle_id)
      .ok_or_else(|| anyhow::anyhow!("Bundle not found: {bundle_id}"))?;

    let assets = self
      .bundle_graph
      .get_bundle_assets_in_source_order(bundle)?;

    let mut hoisted_imports: Vec<String> = Vec::new();
    let mut entry_contents = String::new();
    let mut css_code_map: HashMap<String, String> = HashMap::new();

    for asset in &assets {
      entry_contents.push_str(&format!("@import \"{}\";\n", asset.id));

      // Unresolvable sync deps are external @imports (e.g. Google Fonts); hoist them.
      // Track per-asset so we don't strip imports belonging to other assets.
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
          let import_stmt = format!("@import \"{}\";", dep.specifier);
          if !hoisted_imports.contains(&import_stmt) {
            hoisted_imports.push(import_stmt);
          }
          asset_external_specifiers.push(dep.specifier.clone());
        }
      }

      let db_key = asset.content_key.as_deref().unwrap_or(&asset.id);
      let css_bytes = self.context.db.get(db_key)?.unwrap_or_default();
      let css_code = String::from_utf8(css_bytes)
        .map_err(|e| anyhow::anyhow!("Asset {} CSS is not valid UTF-8: {e}", asset.id))?;

      // Strip external @imports before bundling to prevent unresolvable-URL errors.
      let filtered_css = if asset_external_specifiers.is_empty() {
        css_code
      } else {
        filter_external_imports(&css_code, &asset_external_specifiers)
      };

      css_code_map.insert(asset.id.clone(), filtered_css);
    }

    // Use a reserved prefix for the synthetic entry key to avoid collisions with asset IDs.
    let entry_path = format!("__atlaspack_entry_{}.css", bundle_id);
    css_code_map.insert(entry_path.clone(), entry_contents);

    let provider = InMemoryCssProvider::new(css_code_map);
    let mut bundler = Bundler::new(&provider, None, ParserOptions::default());
    let mut stylesheet = bundler
      .bundle(Path::new(&entry_path))
      .map_err(|e| anyhow::anyhow!("lightningcss bundling failed: {:?}", e))?;

    // CSS Module tree-shaking (production-only, post-bundling AST).
    let mut warnings: Vec<atlaspack_core::types::Diagnostic> = Vec::new();
    if bundle.env.should_optimize {
      for asset in &assets {
        if let Some(used_syms) = self.bundle_graph.get_used_symbols(&asset.id) {
          if used_syms.contains("*") {
            // Wildcard import: retain all classes.
            continue;
          }
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
          apply_css_module_tree_shaking_ast(&mut stylesheet.rules, asset, &used_syms);
        }
      }
    }

    let result = stylesheet
      .to_css(PrinterOptions::default())
      .map_err(|e| anyhow::anyhow!("lightningcss printing failed: {:?}", e))?;
    let mut css = result.code;

    if !hoisted_imports.is_empty() {
      let hoisted = hoisted_imports.join("\n");
      css = format!("{hoisted}\n{css}");
    }

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

/// Strips `@import` lines for any of the given external specifiers.
fn filter_external_imports(css: &str, external_specifiers: &[String]) -> String {
  css
    .lines()
    .filter(|line| {
      let trimmed = line.trim();
      if !trimmed.starts_with("@import") {
        return true;
      }
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

  struct TestBundleGraph {
    bundles: Vec<Bundle>,
    assets_by_bundle: HashMap<String, Vec<Asset>>,
    deps_by_asset: HashMap<String, Vec<Dependency>>,
    resolved: HashMap<String, Asset>,
    skipped: HashSet<String>,
    used_symbols_by_asset: HashMap<String, HashSet<String>>,
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

  fn make_asset(id: &str) -> Asset {
    Asset {
      id: id.to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
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

    let ext_dep = make_dependency(ext_url, Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_1".to_string(), vec![ext_dep]);
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

    assert!(
      !output.contains("@import \"asset_2\""),
      "Internal import should be compiled away"
    );

    let matches: Vec<_> = output.matches(".asset2").collect();
    assert_eq!(
      matches.len(),
      1,
      "Content of asset_2 should appear exactly once (deduplicated)"
    );
  }

  #[test]
  fn handles_bundle_id_colliding_with_asset_id() {
    let db = make_db();
    db.put("foo", b".foo { color: blue; }").unwrap();

    let asset = make_asset("foo");
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

    assert!(
      output.contains(".foo"),
      "Asset content should be present even if bundle ID matches asset ID"
    );
  }

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
  }

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

  #[test]
  fn tree_shaking_preserves_non_module_selectors() {
    let css = ".foo_abc { color: red; } body { margin: 0; }";
    let all: HashSet<String> = [".foo_abc"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = HashSet::new();

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

  #[test]
  fn tree_shaking_wildcard_retains_all_classes() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
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

  #[test]
  fn tree_shaking_is_skipped_in_dev_mode() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    let asset = make_css_module_asset(
      "asset_dev",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      false,
    );
    let used: HashSet<String> = ["foo_abc"].iter().map(|s| s.to_string()).collect();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present in dev mode output; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be present in dev mode output (no tree-shaking); got: {output:?}"
    );
  }

  #[test]
  fn tree_shaking_is_applied_in_production_mode() {
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    let asset = make_css_module_asset(
      "asset_prod",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true,
    );
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

  #[test]
  fn wildcard_import_disables_tree_shaking_in_package() {
    let db = make_db();
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    db.put("asset_wildcard", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_wildcard",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true,
    );

    let bundle = make_bundle("bundle_w", vec!["asset_wildcard"]);
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_w".to_string(), vec![asset]);

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

  #[test]
  fn default_import_disables_tree_shaking() {
    use atlaspack_core::types::Symbol;

    let db = make_db();
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    db.put("asset_default", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_default",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true,
    );

    let bundle = make_bundle("bundle_d", vec!["asset_default"]);
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_d".to_string(), vec![asset.clone()]);

    let mut used_syms = HashSet::new();
    used_syms.insert("default".to_string());
    used_syms.insert("foo".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_default".to_string(), used_syms);

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

  #[test]
  fn tree_shaking_no_op_for_asset_without_symbols() {
    let css = ".plain { color: red; }";
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

  #[test]
  fn tree_shaking_no_op_for_asset_with_empty_symbols() {
    let css = ".plain { color: red; }";
    let asset = make_css_module_asset("asset_empty_syms", vec![], true);
    let used: HashSet<String> = HashSet::new();

    let output = apply_css_module_tree_shaking(css, &asset, &used);

    assert_eq!(
      output, css,
      "asset with empty symbols must be returned unchanged"
    );
  }

  #[test]
  fn tree_shaking_retains_composed_from_class_when_in_used_symbols() {
    let css = ".foo_abc { color: red; } .bar_def { font-weight: bold; }";
    let asset = make_css_module_asset(
      "asset_composes",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true,
    );
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

  #[test]
  fn tree_shaking_retains_empty_media_block_after_removing_nested_rule() {
    let css = "@media (min-width: 500px) { .unused_xyz { color: red; } }";
    let all: HashSet<String> = [".unused_xyz"].iter().map(|s| s.to_string()).collect();
    let used: HashSet<String> = HashSet::new();

    let output = remove_unused_class_rules(css, &all, &used);

    assert!(
      !output.contains(".unused_xyz"),
      "unused class inside @media must be removed by AST shaker; got: {output:?}"
    );
  }

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
    let open_braces = output.chars().filter(|&c| c == '{').count();
    let close_braces = output.chars().filter(|&c| c == '}').count();
    assert_eq!(
      open_braces, close_braces,
      "output must have balanced braces (no corruption from comment brace); got: {output:?}"
    );
  }

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
  }

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

  #[test]
  fn internal_import_missing_from_bundle_is_not_resolved() {
    let db = make_db();
    db.put("asset_1", b"@import \"asset_2\";").unwrap();
    db.put("asset_2", b".asset2 {}").unwrap();

    let asset1 = make_asset("asset_1");
    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
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

    assert!(
      !output.contains(".asset2"),
      "Content of asset_2 should be missing because it is not in the bundle"
    );
  }

  #[test]
  fn tree_shaking_removes_unused_class_inside_container_query() {
    let css = "@container sidebar (min-width: 700px) { .foo_abc { color: red; } .bar_def { color: blue; } }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
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

  #[test]
  fn tree_shaking_removes_unused_class_inside_scope_rule() {
    let css = "@scope (.card) { .foo_abc { color: red; } .bar_def { color: blue; } }";
    let all: HashSet<String> = [".foo_abc", ".bar_def"]
      .iter()
      .map(|s| s.to_string())
      .collect();
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

  #[test]
  fn external_specifier_stripping_is_scoped_to_per_asset() {
    let db = make_db();

    db.put("asset_1", b".a { color: red; }").unwrap();
    let ext_url = "https://fonts.googleapis.com/css?family=Roboto";
    db.put(
      "asset_2",
      format!("@import \"{ext_url}\";\n.b {{ color: blue; }}").as_bytes(),
    )
    .unwrap();

    let asset1 = make_asset("asset_1");
    let asset2 = make_asset("asset_2");
    let bundle = make_bundle("bundle_1", vec!["asset_1", "asset_2"]);

    let ext_dep = make_dependency(ext_url, Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset1, asset2]);
    graph
      .deps_by_asset
      .insert("asset_2".to_string(), vec![ext_dep]);
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

    assert!(
      output.contains(".a"),
      "asset_1 rule must be present; got: {output:?}"
    );
    assert!(
      output.contains(".b"),
      "asset_2 rule must be present; got: {output:?}"
    );

    let import_stmt = format!("@import \"{ext_url}\";");
    let occurrences: Vec<_> = output.matches(&import_stmt).collect();
    assert_eq!(
      occurrences.len(),
      1,
      "External @import must appear exactly once; got: {output:?}"
    );
  }

  // TODO: use a dedicated is_css_module field once added to Asset.
  #[test]
  fn plain_css_asset_is_not_pruned_even_with_matching_selector_name() {
    let css = ".foo_abc { color: red; }";
    let plain_asset = Asset {
      id: "plain_asset".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment {
        should_optimize: true,
        ..Environment::default()
      }),
      symbols: None,
      ..Asset::default()
    };

    let used: HashSet<String> = HashSet::new();

    let output = apply_css_module_tree_shaking(css, &plain_asset, &used);

    assert_eq!(
      output, css,
      "plain CSS asset (symbols = None) must never be pruned, \
       even when its selector name collides with a CSS Module's local name; got: {output:?}"
    );
    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be present in plain CSS output; got: {output:?}"
    );
  }

  // Integration test: composed-from class is retained when only the composing class
  // is in used_symbols, exercised via CssPackager::package().
  #[test]
  fn composes_retention_when_only_composing_class_is_in_used_symbols() {
    let db = make_db();

    let css = ".foo_abc { composes: bar_def; color: red; } .bar_def { font-weight: bold; }";
    db.put("asset_composes_int", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_composes_int",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true,
    );

    let mut bundle = make_bundle("bundle_composes", vec!["asset_composes_int"]);
    bundle.env.should_optimize = true;

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_composes".to_string(), vec![asset.clone()]);

    let mut used_syms = HashSet::new();
    used_syms.insert("foo".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_composes_int".to_string(), used_syms);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_composes")
      .expect("package() must succeed");
    let output = output_string(&result);

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc (composing class, explicitly used) must be retained; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def (composed-from class) must be retained even though 'bar' is not in \
       used_symbols — it is required at runtime by .foo_abc's composes declaration; \
       got: {output:?}"
    );
  }

  #[test]
  fn composes_retention_multiple_local_classes() {
    let db = make_db();
    let css = ".main { composes: a b; color: red; } .a { color: blue; } .b { color: green; }";
    db.put("asset_multi", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_multi",
      vec![("main", "main"), ("a", "a"), ("b", "b")],
      true,
    );

    let mut bundle = make_bundle("bundle_multi", vec!["asset_multi"]);
    bundle.env.should_optimize = true;

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_multi".to_string(), vec![asset.clone()]);

    let mut used_syms = HashSet::new();
    used_syms.insert("main".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_multi".to_string(), used_syms);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_multi")
      .expect("package() must succeed");
    let output = output_string(&result);

    assert!(output.contains(".main"), ".main must be retained");
    assert!(
      output.contains(".a"),
      ".a must be retained (composed by main)"
    );
    assert!(
      output.contains(".b"),
      ".b must be retained (composed by main)"
    );
  }

  #[test]
  fn composes_ignores_global_from() {
    let db = make_db();
    let css = ".main { composes: global-class from global; color: red; } .other { color: blue; }";
    db.put("asset_global", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_global",
      vec![("main", "main"), ("other", "other")],
      true,
    );

    let mut bundle = make_bundle("bundle_global", vec!["asset_global"]);
    bundle.env.should_optimize = true;

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_global".to_string(), vec![asset.clone()]);

    let mut used_syms = HashSet::new();
    used_syms.insert("main".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_global".to_string(), used_syms);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_global")
      .expect("package() must succeed");
    let output = output_string(&result);

    assert!(output.contains(".main"), ".main must be retained");
    assert!(
      !output.contains(".other"),
      ".other is unused and should be removed"
    );
  }

  #[test]
  fn composes_ignores_external_from() {
    let db = make_db();
    let css =
      ".main { composes: ext-class from \"./other.css\"; color: red; } .other { color: blue; }";
    db.put("asset_ext", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_ext",
      vec![("main", "main"), ("other", "other")],
      true,
    );

    let mut bundle = make_bundle("bundle_ext", vec!["asset_ext"]);
    bundle.env.should_optimize = true;

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_ext".to_string(), vec![asset.clone()]);

    let mut used_syms = HashSet::new();
    used_syms.insert("main".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_ext".to_string(), used_syms);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_ext")
      .expect("package() must succeed");
    let output = output_string(&result);

    assert!(output.contains(".main"), ".main must be retained");
    assert!(
      !output.contains(".other"),
      ".other is unused and should be removed"
    );
  }

  #[test]
  fn default_import_emits_structured_warning_in_package_result() {
    use atlaspack_core::types::Symbol;

    let db = make_db();
    let css = ".foo_abc { color: red; } .bar_def { color: blue; }";
    db.put("asset_default", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_default",
      vec![("foo", "foo_abc"), ("bar", "bar_def")],
      true,
    );

    let mut bundle = make_bundle("bundle_d2", vec!["asset_default"]);
    bundle.env.should_optimize = true;
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_d2".to_string(), vec![asset.clone()]);

    let mut used_syms = HashSet::new();
    used_syms.insert("default".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_default".to_string(), used_syms);

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

    assert!(
      output.contains(".foo_abc"),
      ".foo_abc must be retained when default import guard fires; got: {output:?}"
    );
    assert!(
      output.contains(".bar_def"),
      ".bar_def must be retained when default import guard fires; got: {output:?}"
    );

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
