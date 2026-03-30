use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::package_result::{BundleInfo, PackageResult};
use atlaspack_core::types::{Asset, Bundle, BundleBehavior, Diagnostic, Priority};
use lightningcss::bundler::{Bundler, ResolveResult, SourceProvider};
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::custom::{Token, TokenOrValue};
use lightningcss::properties::{Property, PropertyId};
use lightningcss::rules::{CssRule, CssRuleList};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, StyleSheet};
use lightningcss::targets::Browsers;
use lightningcss::traits::ToCss;
use parcel_sourcemap_ext::SourceMap as ParcelSourceMap;

use crate::url_replacer::replace_url_references;
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

  fn resolve(
    &self,
    specifier: &str,
    _originating_file: &Path,
  ) -> std::result::Result<ResolveResult, std::io::Error> {
    Ok(ResolveResult::File(PathBuf::from(specifier)))
  }
}

/// The three sets derived from a CSS Module asset's symbol table, used for tree-shaking.
struct CssModuleSelectors {
  /// CSS selector strings (`.local_name`) for every exported symbol.
  all_module_selectors: HashSet<String>,
  /// CSS selector strings for exported symbols that are actually used.
  used_selectors: HashSet<String>,
}

/// Builds [`CssModuleSelectors`] from an asset's symbol table and the caller-supplied set of
/// used exported names. Returns `None` when the asset has no symbols (i.e. is not a CSS Module).
fn build_module_selector_sets(
  asset: &Asset,
  used_symbols: &HashSet<String>,
) -> Option<CssModuleSelectors> {
  let symbols = asset.symbols.as_ref().filter(|s| !s.is_empty())?;

  let all_module_selectors = symbols.iter().map(|s| format!(".{}", s.local)).collect();

  let used_selectors = symbols
    .iter()
    .filter(|symbol| used_symbols.contains(symbol.exported.as_str()))
    .map(|symbol| format!(".{}", symbol.local))
    .collect();

  Some(CssModuleSelectors {
    all_module_selectors,
    used_selectors,
  })
}

fn remove_unused_from_rule_list<'i>(
  rule_list: &mut CssRuleList<'i>,
  all_module_selectors: &HashSet<String>,
  used_selectors: &HashSet<String>,
) {
  rule_list.0.retain_mut(|rule| {
    // For style rules, remove the rule only when every selector in the group
    // is a module class that is not used.
    if let CssRule::Style(style_rule) = rule {
      let all_unused = style_rule.selectors.0.iter().all(|selector| {
        let selector_str = selector
          .to_css_string(PrinterOptions::default())
          .unwrap_or_default();
        all_module_selectors.contains(&selector_str) && !used_selectors.contains(&selector_str)
      });
      return !all_unused;
    }

    // For container rules (media, supports, etc.), recurse and always retain the rule.
    let child_rules: Option<&mut CssRuleList<'i>> = match rule {
      CssRule::Media(r) => Some(&mut r.rules),
      CssRule::Supports(r) => Some(&mut r.rules),
      CssRule::LayerBlock(r) => Some(&mut r.rules),
      CssRule::StartingStyle(r) => Some(&mut r.rules),
      CssRule::Container(r) => Some(&mut r.rules),
      CssRule::Scope(r) => Some(&mut r.rules),
      _ => None,
    };
    if let Some(children) = child_rules {
      remove_unused_from_rule_list(children, all_module_selectors, used_selectors);
    }
    true
  });
}

/// Applies CSS Module tree-shaking directly on the post-bundling AST in-place,
/// avoiding a redundant parse/print cycle.
fn optimise_css_ast<'i>(
  rules: &mut CssRuleList<'i>,
  assets: &Vec<&Asset>,
  bundle_graph: &Arc<impl BundleGraph>,
  warnings: &mut Vec<Diagnostic>,
) {
  for asset in assets {
    let Some(used_symbols) = bundle_graph.get_used_symbols(&asset.id) else {
      continue;
    };
    if used_symbols.contains("*") {
      // Wildcard import: retain all classes.
      continue;
    }
    if used_symbols.contains("default") && asset_has_default_import(asset, bundle_graph.as_ref()) {
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

    let Some(mut selectors) = build_module_selector_sets(asset, &used_symbols) else {
      return;
    };

    // Expand used_selectors to include classes composed-from by already-used classes.
    // The bundle graph may not always propagate composed-from symbols, so we fall back
    // to parsing `composes:` declarations from the AST.
    expand_composes_selectors(
      rules,
      &selectors.all_module_selectors,
      &mut selectors.used_selectors,
    );

    remove_unused_from_rule_list(
      rules,
      &selectors.all_module_selectors,
      &selectors.used_selectors,
    );
  }
}

/// Extracts the composed class names from a `composes:` declaration.
fn extract_composes_class_names(decl: &Property) -> Option<String> {
  // lightningcss parses `composes: foo;` as `Property::Unparsed` (known property name, value not
  // further typed in non-CSS-modules mode) rather than `Property::Custom`.
  let class_names = match decl {
    Property::Unparsed(unparsed) if unparsed.property_id == PropertyId::Composes => unparsed
      .value
      .0
      .iter()
      .filter_map(|tok| match tok {
        TokenOrValue::Token(Token::Ident(ident)) => Some(ident.as_ref()),
        _ => None,
      })
      .collect::<Vec<_>>(),
    Property::Composes(composes) => composes
      .names
      .iter()
      .map(|n| n.as_ref())
      .collect::<Vec<_>>(),
    _ => return None,
  };
  Some(class_names.join(" "))
}

/// Expands `used_selectors` to a fixed point by following `composes:` declarations in the AST.
fn expand_composes_selectors<'i>(
  rules: &CssRuleList<'i>,
  all_module_selectors: &HashSet<String>,
  used_selectors: &mut HashSet<String>,
) {
  let mut selector_found = true;

  while selector_found {
    selector_found = false;

    for rule in &rules.0 {
      let CssRule::Style(style_rule) = rule else {
        continue;
      };

      let is_rule_used = style_rule.selectors.0.iter().any(|selector| {
        let css = selector
          .to_css_string(PrinterOptions::default())
          .unwrap_or_default();
        used_selectors.contains(&css)
      });

      if !is_rule_used {
        continue;
      }

      for decl in &style_rule.declarations.declarations {
        let Some(value) = extract_composes_class_names(decl) else {
          continue;
        };

        // Parse: `<name>+ [from "<file>"|global]` — take names before the first `from` keyword.
        for name in value
          .split_whitespace()
          .take_while(|tok| !tok.eq_ignore_ascii_case("from"))
        {
          let selector = format!(".{name}");

          if !all_module_selectors.contains(&selector) || used_selectors.contains(&selector) {
            continue;
          }

          used_selectors.insert(selector);
          selector_found = true;
        }
      }
    }
  }
}

/// Constructs a minimal `PackageResult` for an inline style-attribute bundle.
fn build_inline_bundle_result(css: String) -> PackageResult {
  PackageResult {
    bundle_info: BundleInfo {
      bundle_type: "css".to_string(),
      size: css.len() as u64,
      total_assets: 1,
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
    warnings: vec![],
  }
}

/// Constructs the final `PackageResult` for a normal (non-inline) CSS bundle.
fn build_package_result(
  css: String,
  map_bytes: Option<Vec<u8>>,
  total_assets: usize,
  warnings: Vec<Diagnostic>,
) -> PackageResult {
  PackageResult {
    bundle_info: BundleInfo {
      bundle_type: "css".to_string(),
      size: css.len() as u64,
      total_assets: total_assets as u64,
      hash: String::new(),
      hash_references: vec![],
      cache_keys: None,
      is_large_blob: false,
      time: None,
      bundle_contents: Some(css.into_bytes()),
      map_contents: map_bytes,
    },
    config_requests: vec![],
    dev_dep_requests: vec![],
    invalidations: vec![],
    warnings,
  }
}

/// Checks whether any incoming dependency on `asset` imports it via the `default` specifier.
fn asset_has_default_import(asset: &Asset, bundle_graph: &dyn BundleGraph) -> bool {
  bundle_graph
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

    // Inline style attributes (e.g. `style="..."` attributes) only need URL reference replacement.
    if bundle.bundle_behavior == Some(BundleBehavior::Inline)
      && let Some(main_entry_id) = &bundle.main_entry_id
    {
      let all_assets = self.bundle_graph.get_bundle_assets(bundle)?;
      if let Some(entry) = all_assets.iter().find(|a| &a.id == main_entry_id).copied()
        && entry.meta.get("type").and_then(|v| v.as_str()) == Some("attr")
      {
        let css = entry.code.as_str().unwrap_or("");
        let output = url_replacer::replace_url_references(
          css,
          bundle,
          self.bundle_graph.as_ref(),
          &self.context.db,
          &self.context.output_dir,
        )?;
        return Ok(build_inline_bundle_result(output));
      }
    }

    let assets = self
      .bundle_graph
      .get_bundle_assets_in_source_order(bundle)?;

    let project_root_str = self.context.project_root.to_string_lossy().into_owned();
    let mut hoisted_imports: Vec<String> = Vec::new();
    let mut entry_contents = String::new();
    let mut css_code_map: HashMap<String, String> = HashMap::new();

    for asset in &assets {
      entry_contents.push_str(&format!("@import \"{}\";\n", asset.id));

      let external_imports = collect_external_imports(
        asset,
        bundle,
        self.bundle_graph.as_ref(),
        &mut hoisted_imports,
      )?;

      let db_key = asset.content_key.as_deref().unwrap_or(&asset.id);
      let css_bytes = self.context.db.get(db_key)?.unwrap_or_default();
      let mut css_code = String::from_utf8(css_bytes)
        .map_err(|e| anyhow::anyhow!("Asset {} CSS is not valid UTF-8: {e}", asset.id))?;

      // Strip external @imports before bundling to prevent unresolvable-URL errors.
      if !external_imports.is_empty() {
        css_code =
          filter_external_imports(&css_code, &external_imports, bundle.env.should_optimize);
      };

      // Replace CSS variable references with resolved symbol names.
      if asset.meta.get("hasReferences").and_then(|v| v.as_bool()) == Some(true) {
        css_code = apply_css_var_substitution(css_code, asset, bundle, self.bundle_graph.as_ref())?;
      };

      // Append inline source mapping comment when requested and available.
      if bundle.env.source_map.is_some()
        && let Some(ref asset_map) = asset.map
        && let Ok(data_url) = asset_map.clone().to_data_url(None)
      {
        let separator = if css_code.ends_with('\n') { "" } else { "\n" };
        css_code = format!("{css_code}{separator}/*# sourceMappingURL={data_url} */\n");
      };

      css_code_map.insert(asset.id.clone(), css_code);
    }

    // Use a reserved prefix for the synthetic entry key to avoid collisions with asset IDs.
    let entry_path = format!("__atlaspack_entry_{bundle_id}.css");
    css_code_map.insert(entry_path.clone(), entry_contents);

    let mut source_map: Option<ParcelSourceMap> = if bundle.env.source_map.is_some() {
      Some(ParcelSourceMap::new(&project_root_str))
    } else {
      None
    };

    let provider = InMemoryCssProvider::new(css_code_map);
    let mut bundler = Bundler::new(&provider, source_map.as_mut(), ParserOptions::default());
    let mut stylesheet = bundler
      .bundle(Path::new(&entry_path))
      .map_err(|e| anyhow::anyhow!("lightningcss bundling failed: {:?}", e))?;

    let mut warnings: Vec<Diagnostic> = Vec::new();

    if bundle.env.should_optimize {
      optimise_css_ast(
        &mut stylesheet.rules,
        &assets,
        &self.bundle_graph,
        &mut warnings,
      );
    }

    let targets = Browsers::from_browserslist(["last 2 Chrome versions"])
      .expect("Failed to parse browserslist")
      .unwrap_or_default()
      .into();

    let printer_options = PrinterOptions {
      targets,
      minify: bundle.env.should_optimize,
      source_map: source_map.as_mut(),
      project_root: self.context.project_root.to_str(),
      ..PrinterOptions::default()
    };

    stylesheet
      .minify(MinifyOptions {
        targets,
        ..Default::default()
      })
      .unwrap();

    let result = stylesheet
      .to_css(printer_options)
      .map_err(|e| anyhow::anyhow!("lightningcss printing failed: {:?}", e))?;
    let css = result.code;

    let css = hoist_imports(&css, &hoisted_imports, bundle, source_map.as_mut())?;

    let mut css = replace_url_references(
      &css,
      bundle,
      self.bundle_graph.as_ref(),
      &self.context.db,
      &self.context.output_dir,
    )?;

    let map_bytes = generate_map_bytes(&mut css, bundle, source_map.as_mut())?;
    Ok(build_package_result(css, map_bytes, assets.len(), warnings))
  }
}

/// Collects `@import` specifiers from the given asset's dependencies that point to external URIs (eg. Google Fonts).
fn collect_external_imports(
  asset: &&Asset,
  bundle: &Bundle,
  bundle_graph: &dyn BundleGraph,
  hoisted_imports: &mut Vec<String>,
) -> Result<Vec<String>> {
  let mut results = Vec::new();
  for dep in bundle_graph.get_dependencies(asset)? {
    let is_skippable = dep.priority != Priority::Sync || bundle_graph.is_dependency_skipped(dep);
    if is_skippable {
      continue;
    }

    let is_resolvable = bundle_graph.get_resolved_asset(dep, bundle)?.is_some();
    if is_resolvable {
      continue;
    }

    let import_stmt = format!("@import \"{}\";", dep.specifier);
    if !hoisted_imports.contains(&import_stmt) {
      hoisted_imports.push(import_stmt);
    }
    results.push(dep.specifier.clone())
  }

  Ok(results)
}

/// Hoists the given `@import` statements to the top of the CSS, above all inlined imports.
fn hoist_imports(
  css: &String,
  hoisted_imports: &[String],
  bundle: &Bundle,
  mut source_map: Option<&mut ParcelSourceMap>,
) -> Result<String> {
  if hoisted_imports.is_empty() {
    return Ok(css.clone());
  }

  let hoisted_count = hoisted_imports.len() as i64;
  let hoisted = hoisted_imports.join("\n");
  let separator = if bundle.env.should_optimize { "" } else { "\n" };
  let result = format!("{hoisted}{separator}{css}");
  if let Some(ref mut source_map) = source_map {
    source_map
      .offset_lines(0, hoisted_count)
      .map_err(|e| anyhow::anyhow!("source map offset_lines failed: {:?}", e))?;
  }

  Ok(result)
}

/// Serialises the source map to bytes if it exists and inserts a mapping comment to the CSS.
fn generate_map_bytes(
  css: &mut String,
  bundle: &Bundle,
  mut source_map: Option<&mut ParcelSourceMap>,
) -> Result<Option<Vec<u8>>> {
  let Some(ref mut source_map) = source_map else {
    return Ok(None);
  };

  let bundle_name = bundle.name.as_deref().unwrap_or("output.css");
  let separator = if css.ends_with('\n') { "" } else { "\n" };

  css.push_str(&format!(
    "{separator}/*# sourceMappingURL={bundle_name}.map */\n"
  ));

  let map_json = source_map
    .to_json(None)
    .map_err(|error| anyhow::anyhow!("source map serialisation failed: {error:?}"))?;

  Ok(Some(map_json.into_bytes()))
}

/// Escapes a string for use as a CSS dashed identifier (custom property name).
/// Mirrors `escapeDashedIdent` from `CSSPackager.ts`, which follows the CSS
/// serialisation algorithm at https://drafts.csswg.org/cssom/#serialize-an-identifier.
fn escape_dashed_ident(name: &str) -> String {
  let mut res = String::with_capacity(name.len());
  for char in name.chars() {
    let code = char as u32;
    match code {
      0 => res.push('\u{FFFD}'),
      0x01..=0x1f | 0x7f => res.push_str(&format!("\\{} ", char as u32)),
      _ if char.is_ascii_alphanumeric() || matches!(char, '_' | '-') || code >= 0x80 => {
        res.push(char)
      }
      _ => res.push_str(&format!("\\{char}")),
    }
  }
  res
}

/// Replaces CSS variable names in `css` with their resolved symbol names from the bundle graph.
/// Used for CSS Modules `composes:` cross-file references where `asset.meta.hasReferences` is true.
fn apply_css_var_substitution(
  css: String,
  asset: &Asset,
  bundle: &Bundle,
  bundle_graph: &dyn BundleGraph,
) -> anyhow::Result<String> {
  let mut replacements: Vec<(String, String)> = Vec::new();

  for dependency in bundle_graph.get_dependencies(asset)? {
    let Some(symbols) = &dependency.symbols else {
      continue;
    };
    let Some(resolved_asset) = bundle_graph.get_resolved_asset(dependency, bundle)? else {
      continue;
    };
    for symbol in symbols {
      // Find the local name of this exported symbol in the resolved asset.
      let resolved_local = resolved_asset
        .symbols
        .as_ref()
        .and_then(|syms| syms.iter().find(|s| s.exported == symbol.exported))
        .map(|s| s.local.clone())
        .unwrap_or_else(|| symbol.exported.clone());

      replacements.push((symbol.local.clone(), escape_dashed_ident(&resolved_local)));
    }
  }

  if replacements.is_empty() {
    return Ok(css);
  }

  // Build a lookup map for single-pass replacement. Using a HashMap rather than sequential
  // `str::replace` calls prevents double-replacing if a resolved name matches another key.
  let replacement_map: HashMap<&str, &str> = replacements
    .iter()
    .map(|(k, v)| (k.as_str(), v.as_str()))
    .collect();

  // Walk the CSS char-by-char. On recognising a CSS ident start, consume the full ident and
  // emit the replacement if one exists; otherwise emit the original. Each position is visited
  // exactly once.
  let mut result = String::with_capacity(css.len());
  let mut remaining = css.as_str();

  while let Some(c) = remaining.chars().next() {
    let is_ident_start = c.is_alphabetic() || matches!(c, '_' | '-') || c as u32 > 0x7f;
    if is_ident_start {
      let ident_end = remaining
        .find(|ch: char| !ch.is_alphanumeric() && !matches!(ch, '_' | '-') && ch as u32 <= 0x7f)
        .unwrap_or(remaining.len());
      let ident = &remaining[..ident_end];
      result.push_str(replacement_map.get(ident).copied().unwrap_or(ident));
      remaining = &remaining[ident_end..];
    } else {
      result.push(c);
      remaining = &remaining[c.len_utf8()..];
    }
  }

  Ok(result)
}

/// Strips `@import` statements whose URL matches any of the given external specifiers.
fn filter_external_imports(css: &str, external_specifiers: &[String], minify: bool) -> String {
  if external_specifiers.is_empty() {
    return css.to_string();
  }

  let Ok(mut stylesheet) = StyleSheet::parse(css, ParserOptions::default()) else {
    // Fall back to the original string on parse error to never silently lose content.
    return css.to_string();
  };

  stylesheet.rules.0.retain(|rule| match rule {
    CssRule::Import(import_rule) => {
      let url: &str = import_rule.url.as_ref();
      !external_specifiers
        .iter()
        .any(|spec| url.contains(spec.as_str()))
    }
    _ => true,
  });

  stylesheet
    .to_css(PrinterOptions {
      minify,
      ..PrinterOptions::default()
    })
    .map(|res| res.code)
    .unwrap_or_else(|_| css.to_string())
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
    Asset, Bundle, BundleBehavior, Dependency, DependencyBuilder, Environment, FileType, Priority,
    SpecifierType, Symbol, Target,
  };
  use pretty_assertions::assert_eq;
  use serde_json::{Value, from_slice, json};

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

  /// Removes rules for unused CSS Module classes from a CSS string via the lightningcss AST.
  /// A grouped selector rule is only removed if ALL selectors in the group are unused module
  /// classes. Falls back to the original string on parse/serialization failure.
  fn remove_unused_class_rules(
    css: &str,
    all_module_selectors: &HashSet<String>,
    used_selectors: &HashSet<String>,
  ) -> String {
    let mut stylesheet = match StyleSheet::parse(css, Default::default()) {
      Ok(ss) => ss,
      Err(_) => return css.to_string(),
    };

    remove_unused_from_rule_list(&mut stylesheet.rules, all_module_selectors, used_selectors);

    match stylesheet.to_css(PrinterOptions::default()) {
      Ok(result) => result.code,
      Err(_) => css.to_string(),
    }
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
    db.put("asset_1", b"@import \"asset_2\";\n.asset1 { color: red; }")
      .unwrap();
    db.put("asset_2", b".asset2 { color: blue; }").unwrap();

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
  fn reserved_entry_prefix_avoids_collision_with_asset_id() {
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
    db.put("normal", b".normal { margin: 0; }").unwrap();

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

  fn make_bundle_with_name(id: &str, entry_asset_ids: Vec<&str>, name: &str) -> Bundle {
    Bundle {
      name: Some(name.to_string()),
      ..make_bundle(id, entry_asset_ids)
    }
  }

  fn make_env_with_source_map() -> Environment {
    use atlaspack_core::types::TargetSourceMapOptions;
    Environment {
      source_map: Some(TargetSourceMapOptions::default()),
      ..Environment::default()
    }
  }

  #[test]
  fn source_map_absent_when_source_map_env_disabled() {
    let db = make_db();
    db.put("asset_sm1", b".foo { color: red; }").unwrap();

    let asset = make_asset("asset_sm1");
    let bundle = make_bundle("bundle_sm1", vec!["asset_sm1"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle);
    graph
      .assets_by_bundle
      .insert("bundle_sm1".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_sm1")
      .expect("package() must succeed");

    assert!(
      result.bundle_info.map_contents.is_none(),
      "map_contents must be None when source_map env option is disabled"
    );

    let css = output_string(&result);
    assert!(
      !css.contains("sourceMappingURL"),
      "CSS output must not contain sourceMappingURL when source maps are disabled; got: {css:?}"
    );
  }

  #[test]
  fn source_map_emitted_when_source_map_env_enabled() {
    let db = make_db();
    db.put("asset_sm2", b".foo { color: red; }").unwrap();

    let asset = Asset {
      id: "asset_sm2".to_string(),
      file_type: FileType::Css,
      env: Arc::new(make_env_with_source_map()),
      ..Asset::default()
    };

    let mut bundle = make_bundle_with_name("bundle_sm2", vec!["asset_sm2"], "output.css");
    bundle.env = make_env_with_source_map();

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle);
    graph
      .assets_by_bundle
      .insert("bundle_sm2".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_sm2")
      .expect("package() must succeed");

    let map_bytes = result
      .bundle_info
      .map_contents
      .as_ref()
      .expect("map_contents must be Some when source maps are enabled");

    let map_json: Value = from_slice(map_bytes).expect("map_contents must be valid JSON");

    assert_eq!(
      map_json["version"],
      json!(3),
      "source map version must be 3; got: {map_json:?}"
    );

    let sources = map_json["sources"]
      .as_array()
      .expect("source map must have a 'sources' array");
    assert!(
      !sources.is_empty(),
      "source map 'sources' must be non-empty; got: {map_json:?}"
    );

    let mappings = map_json["mappings"]
      .as_str()
      .expect("source map must have a 'mappings' string");
    assert!(
      !mappings.is_empty(),
      "source map 'mappings' must be non-empty; got: {map_json:?}"
    );

    let css = output_string(&result);
    assert!(
      css.contains("/*# sourceMappingURL=") && css.contains(".map */"),
      "CSS must contain a sourceMappingURL comment ending with '.map */'; got: {css:?}"
    );
  }

  #[test]
  fn source_map_line_offset_correct_for_hoisted_imports() {
    let db = make_db();
    let ext_url = "https://fonts.googleapis.com/css?family=Roboto";
    let css_content = format!("@import \"{ext_url}\";\n.foo {{ color: red; }}");
    db.put("asset_sm3", css_content.as_bytes()).unwrap();

    let asset = Asset {
      id: "asset_sm3".to_string(),
      file_type: FileType::Css,
      env: Arc::new(make_env_with_source_map()),
      ..Asset::default()
    };

    let mut bundle = make_bundle_with_name("bundle_sm3", vec!["asset_sm3"], "output.css");
    bundle.env = make_env_with_source_map();

    let ext_dep = make_dependency(ext_url, Priority::Sync);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle);
    graph
      .assets_by_bundle
      .insert("bundle_sm3".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_sm3".to_string(), vec![ext_dep]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_sm3")
      .expect("package() must succeed");

    let map_bytes = result
      .bundle_info
      .map_contents
      .as_ref()
      .expect("map_contents must be Some when source maps are enabled");

    let map_json: Value = from_slice(map_bytes).expect("map_contents must be valid JSON");

    let mappings = map_json["mappings"]
      .as_str()
      .expect("source map must have a 'mappings' string");

    // Semicolon indicates the first generated line (containing the @import) has no mappings.
    assert!(
      mappings.starts_with(';'),
      "mappings must start with ';' to indicate the first line is skipped due to hoisting; \
       got mappings: {mappings:?}"
    );
  }

  #[test]
  fn source_map_sources_contain_expected_paths() {
    let db = make_db();
    db.put("asset_sm4", b".bar { margin: 0; }").unwrap();

    let asset = Asset {
      id: "asset_sm4".to_string(),
      file_type: FileType::Css,
      env: Arc::new(make_env_with_source_map()),
      ..Asset::default()
    };

    let mut bundle = make_bundle_with_name("bundle_sm4", vec!["asset_sm4"], "output.css");
    bundle.env = make_env_with_source_map();

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle);
    graph
      .assets_by_bundle
      .insert("bundle_sm4".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_sm4")
      .expect("package() must succeed");

    let map_bytes = result
      .bundle_info
      .map_contents
      .as_ref()
      .expect("map_contents must be Some when source maps are enabled");

    let map_json: Value = from_slice(map_bytes).expect("map_contents must be valid JSON");

    let sources = map_json["sources"]
      .as_array()
      .expect("source map must have a 'sources' array");

    assert!(
      !sources.is_empty(),
      "source map 'sources' must be non-empty; got: {map_json:?}"
    );

    let has_asset_source = sources
      .iter()
      .any(|s| s.as_str().map(|p| p.contains("asset_sm4")).unwrap_or(false));
    assert!(
      has_asset_source,
      "source map 'sources' must contain a path referencing 'asset_sm4'; got sources: {sources:?}"
    );
  }

  #[test]
  fn default_import_emits_structured_warning_in_package_result() {
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

  #[test]
  fn source_map_handles_multiple_assets() {
    let db = make_db();
    db.put("asset_a", b".a { color: red; }").unwrap();
    db.put("asset_b", b".b { color: blue; }").unwrap();

    let asset_a = Asset {
      id: "asset_a".to_string(),
      file_type: FileType::Css,
      env: Arc::new(make_env_with_source_map()),
      ..Asset::default()
    };
    let asset_b = Asset {
      id: "asset_b".to_string(),
      file_type: FileType::Css,
      env: Arc::new(make_env_with_source_map()),
      ..Asset::default()
    };

    let mut bundle = make_bundle_with_name("bundle_ab", vec!["asset_a", "asset_b"], "output.css");
    bundle.env = make_env_with_source_map();

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_ab".to_string(), vec![asset_a, asset_b]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_ab")
      .expect("package() must succeed");

    let map_bytes = result
      .bundle_info
      .map_contents
      .as_ref()
      .expect("map_contents must be Some");

    let map_json: Value = from_slice(map_bytes).unwrap();
    let sources = map_json["sources"].as_array().unwrap();

    let has_a = sources
      .iter()
      .any(|s| s.as_str().unwrap().contains("asset_a"));
    let has_b = sources
      .iter()
      .any(|s| s.as_str().unwrap().contains("asset_b"));

    assert!(has_a, "source map must contain asset_a");
    assert!(has_b, "source map must contain asset_b");
  }

  #[test]
  fn source_map_offsets_not_updated_after_url_replacement() {
    // This test documents a known limitation: URL replacement happens after source map generation,
    // so if the URL length changes, column offsets in the source map will be slightly incorrect
    // for the remainder of the line. We verify this by showing that two bundles with different
    // URL replacement lengths produce identical source map mappings.

    let db = make_db();
    let css_content = ".foo { background: url(icon.png); }";
    db.put("asset_url", css_content.as_bytes()).unwrap();

    // Common setup
    let make_package_result = |target_dist_dir: &str| -> PackageResult {
      let asset = Asset {
        id: "asset_url".to_string(),
        file_type: FileType::Css,
        env: Arc::new(make_env_with_source_map()),
        ..Asset::default()
      };

      // CSS Bundle at /dist/css/style.css
      let mut bundle = make_bundle_with_name("bundle_css", vec!["asset_url"], "style.css");
      bundle.env = make_env_with_source_map();
      bundle.target = Target {
        dist_dir: PathBuf::from("/dist/css"),
        ..Target::default()
      };

      // Image Bundle at `target_dist_dir` (e.g. /dist/css/ (same dir) or /dist/img/ (nested))
      // We simulate the resolved asset being in a different bundle.
      let image_asset = Asset {
        id: "asset_img".to_string(),
        file_type: FileType::Png,
        ..Asset::default()
      };
      let image_bundle = Bundle {
        id: "bundle_img".to_string(),
        bundle_type: FileType::Png,
        entry_asset_ids: vec!["asset_img".to_string()],
        name: Some("icon.png".to_string()),
        target: Target {
          dist_dir: PathBuf::from(target_dist_dir),
          ..Target::default()
        },
        ..make_bundle("bundle_img", vec!["asset_img"])
      };

      let dep = make_dependency("icon.png", Priority::Sync);

      let mut graph = TestBundleGraph::new();
      graph.bundles.push(bundle.clone());
      graph.bundles.push(image_bundle.clone());
      graph
        .assets_by_bundle
        .insert("bundle_css".to_string(), vec![asset.clone()]);
      graph
        .assets_by_bundle
        .insert("bundle_img".to_string(), vec![image_asset.clone()]);
      graph
        .deps_by_asset
        .insert("asset_url".to_string(), vec![dep.clone()]);
      graph.resolved.insert("icon.png".to_string(), image_asset);

      let packager = CssPackager::new(
        CssPackagingContext {
          db: db.clone(),
          project_root: PathBuf::from("/tmp"),
          output_dir: PathBuf::from("/dist/css"), // Outputting to css dir
        },
        Arc::new(graph),
      );

      packager
        .package("bundle_css")
        .expect("package() must succeed")
    };

    // Case 1: Image is in same directory (/dist/css). Relative URL should be "icon.png" (len 8).
    // Original "icon.png" is len 8. replacement is len 8.
    let result_short = make_package_result("/dist/css");
    let css_short = output_string(&result_short);

    // Case 2: Image is in deep directory (/dist/assets/images). Relative URL should be "../assets/images/icon.png" (longer).
    let result_long = make_package_result("/dist/assets/images");
    let css_long = output_string(&result_long);

    assert_ne!(
      css_short, css_long,
      "CSS output should differ due to URL paths"
    );

    let map_short =
      from_slice::<Value>(result_short.bundle_info.map_contents.as_ref().unwrap()).unwrap();
    let map_long =
      from_slice::<Value>(result_long.bundle_info.map_contents.as_ref().unwrap()).unwrap();

    assert_eq!(
      map_short["mappings"], map_long["mappings"],
      "Mappings should be identical despite URL length difference (proving offsets are not updated)"
    );
  }

  #[test]
  fn source_map_composes_per_asset_input_map() {
    let db = make_db();
    let css_content = ".foo { color: red; }";
    db.put("asset_input_map", css_content.as_bytes()).unwrap();

    // Create a SourceMap that represents an upstream transformation (e.g. from Sass)
    // We map the CSS content to "foo.scss"
    let mut sm = atlaspack_core::types::SourceMap::new(Path::new("/tmp"));
    sm.add_empty_map("foo.scss", css_content, 0)
      .expect("failed to add empty map");

    let asset = Asset {
      id: "asset_input_map".to_string(),
      file_type: FileType::Css,
      env: Arc::new(make_env_with_source_map()),
      map: Some(sm),
      ..Asset::default()
    };

    let mut bundle =
      make_bundle_with_name("bundle_input_map", vec!["asset_input_map"], "output.css");
    bundle.env = make_env_with_source_map();

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle);
    graph
      .assets_by_bundle
      .insert("bundle_input_map".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_input_map")
      .expect("package() must succeed");

    let map_bytes = result
      .bundle_info
      .map_contents
      .as_ref()
      .expect("map_contents must be Some when source maps are enabled");

    let map_json: Value = from_slice(map_bytes).expect("map_contents must be valid JSON");

    let sources = map_json["sources"]
      .as_array()
      .expect("source map must have a 'sources' array");

    let has_original_source = sources
      .iter()
      .any(|s| s.as_str().map(|p| p.contains("foo.scss")).unwrap_or(false));

    assert!(
      has_original_source,
      "source map must contain original source 'foo.scss' from input map; got sources: {sources:?}"
    );
  }

  #[test]
  fn escape_dashed_ident_normal_chars() {
    assert_eq!(escape_dashed_ident("normal"), "normal");
    assert_eq!(escape_dashed_ident("with-dash"), "with-dash");
    assert_eq!(escape_dashed_ident("with_under"), "with_under");
    assert_eq!(escape_dashed_ident("café"), "café"); // non-ASCII passthrough
  }

  #[test]
  fn escape_dashed_ident_special_chars() {
    assert_eq!(escape_dashed_ident("with space"), "with\\ space");
    assert_eq!(escape_dashed_ident("with.dot"), "with\\.dot");
  }

  #[test]
  fn escape_dashed_ident_control_chars() {
    // Code point 0 → replacement character
    assert_eq!(escape_dashed_ident("\0"), "\u{FFFD}");
    // Control chars get hex escape
    let result = escape_dashed_ident("\x01");
    assert!(
      result.starts_with("\\1 "),
      "Control char must be hex-escaped; got: {result:?}"
    );
  }

  #[test]
  fn filter_external_imports_strips_media_query_import() {
    // Also includes a comment with a semicolon inside to verify the AST-based
    // implementation correctly ignores semicolons inside comments.
    let css = "@import \"https://fonts.googleapis.com/css\" /* font; load */ screen, print;\n.local { color: red; }";
    let external = vec!["https://fonts.googleapis.com".to_string()];
    let result = filter_external_imports(css, &external, false);
    assert!(
      !result.contains("@import"),
      "media-query-qualified external @import (with comment containing semicolon) must be stripped; got: {result:?}"
    );
    assert!(
      result.contains(".local"),
      "local rules must be preserved; got: {result:?}"
    );
  }

  #[test]
  fn filter_external_imports_strips_multiline_import() {
    let css = "@import\n  \"https://example.com/ext.css\"\n  screen;\n.keep { color: blue; }";
    let external = vec!["https://example.com/ext.css".to_string()];
    let result = filter_external_imports(css, &external, false);
    assert!(
      !result.contains("@import"),
      "multi-line external @import must be stripped; got: {result:?}"
    );
    assert!(result.contains(".keep"));
  }

  #[test]
  fn filter_external_imports_preserves_local_imports() {
    let css = "@import \"./local.css\";\n.local { color: green; }";
    let external = vec!["https://external.com".to_string()];
    let result = filter_external_imports(css, &external, false);
    assert!(
      result.contains("@import"),
      "local @import must NOT be stripped; got: {result:?}"
    );
    assert!(result.contains(".local"));
  }

  #[test]
  fn filter_external_imports_minifies_output_when_flag_is_true() {
    // A non-matching specifier forces the full parse/print path while keeping all rules.
    let css = ".keep {\n  color: red;\n  font-size: 1rem;\n}";
    let external = vec!["https://fonts.googleapis.com".to_string()];
    let result = filter_external_imports(css, &external, true);
    // Minified output must not contain newlines or indentation whitespace.
    assert!(
      !result.contains('\n'),
      "minified output must not contain newlines; got: {result:?}"
    );
    assert!(
      !result.contains("  "),
      "minified output must not contain indentation whitespace; got: {result:?}"
    );
    assert!(
      result.contains("color:red"),
      "minified output must collapse 'color: red' to 'color:red'; got: {result:?}"
    );
  }

  #[test]
  fn filter_external_imports_preserves_whitespace_when_flag_is_false() {
    // A non-matching specifier forces the full parse/print path while keeping all rules.
    let css = ".keep {\n  color: red;\n}";
    let external = vec!["https://fonts.googleapis.com".to_string()];
    let result = filter_external_imports(css, &external, false);
    // Non-minified output must keep declarations on separate indented lines.
    assert!(
      result.contains('\n'),
      "non-minified output must contain newlines; got: {result:?}"
    );
    assert!(
      result.contains("color: red"),
      "non-minified output must preserve space in 'color: red'; got: {result:?}"
    );
  }

  #[test]
  fn escape_dashed_ident_starts_with_digit() {
    assert_eq!(escape_dashed_ident("123"), "123");
    assert_eq!(escape_dashed_ident("1a"), "1a");
  }

  #[test]
  fn escape_dashed_ident_starts_with_dash_digit() {
    assert_eq!(escape_dashed_ident("-123"), "-123");
  }

  #[test]
  fn inline_style_attribute_short_circuit() {
    let db = make_db();
    let css = "background: url(image.png)";
    db.put("attr_asset", css.as_bytes()).unwrap();

    let mut asset = make_asset("attr_asset");
    asset.meta.insert("type".to_string(), "attr".into());
    asset.bundle_behavior = Some(BundleBehavior::Inline);
    // Populate asset.code because the short-circuit reads from it directly (skipping DB)
    asset.code = css.to_string().into();

    // Create a dependency for the URL
    let dep = make_dependency("image.png", Priority::Sync);

    let mut bundle = make_bundle("attr_bundle", vec!["attr_asset"]);
    bundle.bundle_behavior = Some(BundleBehavior::Inline);
    bundle.main_entry_id = Some("attr_asset".to_string());

    // In `make_bundle`, env is default, so output format is `Global`.
    // The url replacer uses `find_relative_path`.
    // We need to ensure the graph returns the resolved asset for the dependency.

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("attr_bundle".to_string(), vec![asset.clone()]);
    graph
      .deps_by_asset
      .insert("attr_asset".to_string(), vec![dep]);

    // Mock resolved asset so URL replacer works
    let image_asset = make_asset("image_asset");
    // Also need a bundle for the image asset if we want find_relative_path to find it,
    // OR we can make it inline so it gets base64 encoded.
    // If we want it to be a URL replacement to another file, that file needs to be in a bundle.
    // Let's make the image asset inline for simplicity to test the replacement path?
    // Actually, `replace_url_references` handles inline assets by base64 encoding them.
    // Let's test that path as it's simpler to setup in this mock graph.
    let mut image_asset = image_asset;
    image_asset.bundle_behavior = Some(BundleBehavior::Inline);
    image_asset.content_key = Some("image_content".to_string());
    db.put("image_content", b"fake-image-data").unwrap();

    graph.resolved.insert("image.png".to_string(), image_asset);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("attr_bundle")
      .expect("package() must succeed");
    let output = output_string(&result);

    // It should be just the property value with replaced URL, not wrapped in a rule
    // assert!(output.contains("background: url("), "Output should contain the CSS property");
    assert!(
      output.contains("background: url("),
      "Output should contain the CSS property; got: {}",
      output
    );
    assert!(!output.contains("image.png"), "URL should be replaced");
    assert!(
      output.contains("data:application/octet-stream,"),
      "Should contain percent-encoded data URI; got: {}",
      output
    );
  }

  #[test]
  fn apply_css_var_substitution_replaces_references() {
    let db = make_db();
    // Asset 1 composes 'bar' from Asset 2
    let css = ".foo { composes: bar from \"./other.css\"; color: red; }";
    db.put("asset_1", css.as_bytes()).unwrap();
    db.put("asset_2", b".bar { color: blue; }").unwrap();

    let mut asset1 = make_asset("asset_1");
    asset1.meta.insert("hasReferences".to_string(), true.into());

    let asset2 = make_asset("asset_2");

    // Asset 2 exports 'bar' as 'bar_hashed'
    let asset2_symbols = vec![Symbol {
      exported: "bar".to_string(),
      local: "bar_hashed".to_string(),
      loc: None,
      is_weak: false,
      is_esm_export: true,
      self_referenced: false,
      is_static_binding_safe: true,
    }];
    let mut asset2_with_symbols = asset2.clone();
    asset2_with_symbols.symbols = Some(asset2_symbols);

    // Dependency from asset 1 to asset 2
    let mut dep = make_dependency("./other.css", Priority::Sync);
    dep.symbols = Some(vec![Symbol {
      exported: "bar".to_string(),
      local: "bar".to_string(), // local name in asset 1
      loc: None,
      is_weak: false,
      is_esm_export: true,
      self_referenced: false,
      is_static_binding_safe: true,
    }]);

    let bundle = make_bundle("bundle_1", vec!["asset_1"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    // We only care about asset 1 being packaged here, but asset 2 is needed for resolution
    graph
      .assets_by_bundle
      .insert("bundle_1".to_string(), vec![asset1.clone()]);
    graph.deps_by_asset.insert("asset_1".to_string(), vec![dep]);
    graph
      .resolved
      .insert("./other.css".to_string(), asset2_with_symbols);

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

    // "bar" should be replaced by "bar_hashed"
    assert!(
      output.contains("bar_hashed"),
      "CSS var substitution should happen: 'bar' -> 'bar_hashed'"
    );
  }

  // When "foo" is a prefix of "foobar" and both are replacement keys, a naive
  // sequential str::replace would corrupt "foobar" into "foo_resolvedbar".
  // The char-by-char ident scanner must treat each token atomically.
  #[test]
  fn apply_css_var_substitution_no_double_replace_for_overlapping_keys() {
    let db = make_db();
    let css = ".a { composes: foo foobar; color: red; }";
    db.put("asset_overlap", css.as_bytes()).unwrap();

    let mut asset = make_asset("asset_overlap");
    asset.meta.insert("hasReferences".to_string(), true.into());

    let mut dep = make_dependency("./other.css", Priority::Sync);
    dep.symbols = Some(vec![
      Symbol {
        exported: "foo".to_string(),
        local: "foo".to_string(),
        loc: None,
        is_weak: false,
        is_esm_export: false,
        self_referenced: false,
        is_static_binding_safe: true,
      },
      Symbol {
        exported: "foobar".to_string(),
        local: "foobar".to_string(),
        loc: None,
        is_weak: false,
        is_esm_export: false,
        self_referenced: false,
        is_static_binding_safe: true,
      },
    ]);

    let mut resolved = make_asset("other");
    resolved.symbols = Some(vec![
      Symbol {
        exported: "foo".to_string(),
        local: "foo_resolved".to_string(),
        loc: None,
        is_weak: false,
        is_esm_export: false,
        self_referenced: false,
        is_static_binding_safe: true,
      },
      Symbol {
        exported: "foobar".to_string(),
        local: "foobar_resolved".to_string(),
        loc: None,
        is_weak: false,
        is_esm_export: false,
        self_referenced: false,
        is_static_binding_safe: true,
      },
    ]);

    let bundle = make_bundle("bundle_overlap", vec!["asset_overlap"]);
    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_overlap".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_overlap".to_string(), vec![dep]);
    graph.resolved.insert("./other.css".to_string(), resolved);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_overlap").expect("should succeed");
    let output = output_string(&result);

    assert!(
      output.contains("foo_resolved"),
      "'foo' must be replaced with 'foo_resolved'; got: {output:?}"
    );
    assert!(
      output.contains("foobar_resolved"),
      "'foobar' must be replaced with 'foobar_resolved'; got: {output:?}"
    );
    // Sequential str::replace("foo", "foo_resolved") would corrupt "foobar"
    // into "foo_resolvedbar". The single-pass scanner must not produce that.
    assert!(
      !output.contains("foo_resolvedbar"),
      "double-replace corruption must not occur; got: {output:?}"
    );
  }

  // Three-level composes chain (.a → .b → .c) where rules appear in reverse
  // declaration order so that depth > 1 requires the outer fixed-point while
  // loop to restart before .b's own composes: is discovered.
  #[test]
  fn composes_chained_three_levels_retains_all() {
    let db = make_db();
    // Rules deliberately in reverse order so the first pass only discovers .b;
    // the second pass discovers .c via .b's composes.
    let css =
      ".c { color: green; } .b { composes: c; color: blue; } .a { composes: b; color: red; }";
    db.put("asset_chain", css.as_bytes()).unwrap();

    let asset = make_css_module_asset(
      "asset_chain",
      vec![("a", "a"), ("b", "b"), ("c", "c")],
      true,
    );

    let mut bundle = make_bundle("bundle_chain", vec!["asset_chain"]);
    bundle.env.should_optimize = true;

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_chain".to_string(), vec![asset]);

    let mut used_syms = HashSet::new();
    used_syms.insert("a".to_string());
    graph
      .used_symbols_by_asset
      .insert("asset_chain".to_string(), used_syms);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("bundle_chain").expect("should succeed");
    let output = output_string(&result);

    assert!(
      output.contains(".a"),
      ".a (directly used) must be retained; got: {output:?}"
    );
    assert!(
      output.contains(".b"),
      ".b (composed by .a, first expansion pass) must be retained; got: {output:?}"
    );
    assert!(
      output.contains(".c"),
      ".c (composed by .b, second expansion pass) must be retained via fixed-point; got: {output:?}"
    );
  }

  // An inline bundle whose main entry does NOT carry `meta["type"] = "attr"`
  // must fall through to the standard lightningcss bundling path rather than
  // the attr-specific short-circuit.
  #[test]
  fn non_attr_inline_bundle_falls_through_to_standard_path() {
    let db = make_db();
    db.put("inline_asset_notype", b".foo { color: red; }")
      .unwrap();

    let mut asset = make_asset("inline_asset_notype");
    // Inline bundle behavior but no "type": "attr" meta key.
    asset.bundle_behavior = Some(BundleBehavior::Inline);

    let mut bundle = make_bundle("inline_bundle_notype", vec!["inline_asset_notype"]);
    bundle.bundle_behavior = Some(BundleBehavior::Inline);
    bundle.main_entry_id = Some("inline_asset_notype".to_string());

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("inline_bundle_notype".to_string(), vec![asset]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("inline_bundle_notype")
      .expect("package() must succeed");
    let output = output_string(&result);

    assert!(
      output.contains(".foo"),
      "Non-attr inline bundle must be processed by the standard lightningcss path; got: {output:?}"
    );
  }

  #[test]
  fn package_returns_error_for_unknown_bundle_id() {
    let db = make_db();
    let graph = TestBundleGraph::new();

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager.package("nonexistent_bundle_id");
    assert!(
      result.is_err(),
      "package() must return Err for an unknown bundle ID"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
      err_msg.contains("nonexistent_bundle_id"),
      "Error message must name the unknown bundle ID; got: {err_msg:?}"
    );
  }

  #[test]
  fn hoist_imports_minified_no_newline_separator() {
    let db = make_db();
    let ext_url = "https://fonts.googleapis.com/css?family=Roboto";
    let css_content = format!("@import \"{ext_url}\";\nbody {{ color: red; }}");
    db.put("asset_hoist_min", css_content.as_bytes()).unwrap();

    let asset = make_asset("asset_hoist_min");
    let ext_dep = make_dependency(ext_url, Priority::Sync);

    let mut bundle = make_bundle("bundle_hoist_min", vec!["asset_hoist_min"]);
    bundle.env.should_optimize = true;

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_hoist_min".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_hoist_min".to_string(), vec![ext_dep]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_hoist_min")
      .expect("should succeed");
    let output = output_string(&result);

    let import_stmt = format!("@import \"{ext_url}\";");
    let import_end = output
      .find(&import_stmt)
      .expect("hoisted @import must appear in output")
      + import_stmt.len();

    let next_char = output[import_end..].chars().next();
    assert_ne!(
      next_char,
      Some('\n'),
      "In minified mode the hoisted @import must not be followed by a newline; got: {output:?}"
    );
  }

  #[test]
  fn hoist_imports_non_minified_newline_separator() {
    let db = make_db();
    let ext_url = "https://fonts.googleapis.com/css?family=Open+Sans";
    let css_content = format!("@import \"{ext_url}\";\nbody {{ color: blue; }}");
    db.put("asset_hoist_nomin", css_content.as_bytes()).unwrap();

    let asset = make_asset("asset_hoist_nomin");
    let ext_dep = make_dependency(ext_url, Priority::Sync);

    // should_optimize defaults to false via make_bundle
    let bundle = make_bundle("bundle_hoist_nomin", vec!["asset_hoist_nomin"]);

    let mut graph = TestBundleGraph::new();
    graph.bundles.push(bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_hoist_nomin".to_string(), vec![asset]);
    graph
      .deps_by_asset
      .insert("asset_hoist_nomin".to_string(), vec![ext_dep]);

    let packager = CssPackager::new(
      CssPackagingContext {
        db,
        project_root: PathBuf::from("/tmp"),
        output_dir: PathBuf::from("/tmp/dist"),
      },
      Arc::new(graph),
    );

    let result = packager
      .package("bundle_hoist_nomin")
      .expect("should succeed");
    let output = output_string(&result);

    let import_stmt = format!("@import \"{ext_url}\";");
    let import_end = output
      .find(&import_stmt)
      .expect("hoisted @import must appear in output")
      + import_stmt.len();

    let next_char = output[import_end..].chars().next();
    assert_eq!(
      next_char,
      Some('\n'),
      "In non-minified mode the hoisted @import must be followed by a newline; got: {output:?}"
    );
  }
}
