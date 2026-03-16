use std::path::{Component, Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::database::DatabaseRef;
use atlaspack_core::types::{Bundle, BundleBehavior, FileType};

/// Replaces `url()` placeholder tokens emitted by lightningcss with the correct
/// relative output path (or a base64 data URI for inline assets).
///
/// Called after lightningcss bundling in `CssPackager::package()`, on the final
/// output CSS string.
pub fn replace_url_references(
  css: &str,
  bundle: &Bundle,
  bundle_graph: &dyn BundleGraph,
  db: &DatabaseRef,
  output_dir: &Path,
) -> anyhow::Result<String> {
  // Collect all URL dependencies (skip CSS @import deps) for assets in this bundle.
  let bundle_assets = bundle_graph.get_bundle_assets(bundle)?;

  // Build placeholder → dependency map, only for non-CSS-import URL deps.
  let mut placeholder_to_dep = Vec::new();
  for asset in &bundle_assets {
    let deps = bundle_graph.get_dependencies(asset)?;
    for dep in deps {
      if dep.is_css_import {
        continue;
      }
      let token = dep.placeholder.as_deref().unwrap_or(dep.specifier.as_str());
      placeholder_to_dep.push((token.to_string(), dep));
    }
  }

  // Fast path: no URL deps, nothing to replace.
  if placeholder_to_dep.is_empty() {
    return Ok(css.to_string());
  }

  // Filter to only those placeholders actually present in the CSS string.
  let active: Vec<_> = placeholder_to_dep
    .iter()
    .filter(|(token, _)| css.contains(token.as_str()))
    .collect();

  if active.is_empty() {
    return Ok(css.to_string());
  }

  let mut result = css.to_string();

  for (token, dep) in &active {
    let resolved = bundle_graph.get_resolved_asset(dep, bundle)?;

    let replacement = match resolved {
      None => {
        // Unresolvable: fall back to the original specifier.
        dep.specifier.clone()
      }
      Some(asset) => {
        let is_inline = matches!(
          asset.bundle_behavior,
          Some(BundleBehavior::Inline) | Some(BundleBehavior::InlineIsolated)
        );

        if is_inline {
          // Read asset bytes from the DB and encode as a data URI.
          let db_key = asset.content_key.as_deref().unwrap_or(asset.id.as_str());
          let bytes = db.get(db_key)?.unwrap_or_default();
          let mime = mime_for_file_type(&asset.file_type);
          let encoded = BASE64_STANDARD.encode(&bytes);
          format!("data:{mime};base64,{encoded}")
        } else {
          // Find the bundle that owns this asset and compute a relative path.
          find_relative_path(asset.id.as_str(), bundle, bundle_graph, output_dir)
            .unwrap_or_else(|| dep.specifier.clone())
        }
      }
    };

    result = result.replace(token.as_str(), &replacement);
  }

  Ok(result)
}

/// Returns the MIME type string for a given `FileType`.
fn mime_for_file_type(file_type: &FileType) -> &'static str {
  match file_type {
    FileType::Png => "image/png",
    FileType::Jpeg => "image/jpeg",
    FileType::Gif => "image/gif",
    FileType::WebP => "image/webp",
    FileType::Avif => "image/avif",
    FileType::Tiff => "image/tiff",
    FileType::Other(ext) => match ext.as_str() {
      "svg" => "image/svg+xml",
      "woff2" => "font/woff2",
      "woff" => "font/woff",
      "ttf" => "font/ttf",
      "eot" => "application/vnd.ms-fontobject",
      _ => "application/octet-stream",
    },
    _ => "application/octet-stream",
  }
}

/// Finds the bundle containing `asset_id` and computes a forward-slash relative
/// path from the CSS bundle's output directory to that bundle's output file.
/// Returns `None` if no containing bundle is found or if a path cannot be computed.
fn find_relative_path(
  asset_id: &str,
  css_bundle: &Bundle,
  bundle_graph: &dyn BundleGraph,
  output_dir: &Path,
) -> Option<String> {
  let target_bundle = bundle_graph.get_bundles().into_iter().find(|b| {
    // Skip the CSS bundle itself and inline bundles.
    if b.id == css_bundle.id {
      return false;
    }
    if matches!(
      b.bundle_behavior,
      Some(BundleBehavior::Inline) | Some(BundleBehavior::InlineIsolated)
    ) {
      return false;
    }
    // Check if this bundle contains the asset.
    bundle_graph
      .get_bundle_assets(b)
      .ok()
      .map(|assets| assets.iter().any(|a| a.id == asset_id))
      .unwrap_or(false)
  })?;

  let to_name = target_bundle.name.as_deref().filter(|n| !n.is_empty())?;

  // CSS bundle output file path (used to derive the "from" directory).
  let from_name = css_bundle.name.as_deref().unwrap_or("");
  let from_file = output_dir.join(from_name);
  let from_dir = from_file.parent().unwrap_or(output_dir);

  let to_file = target_bundle.target.dist_dir.join(to_name);

  let rel = diff_paths(&to_file, from_dir)?;
  Some(path_to_url_string(&rel))
}

/// Computes a relative path from `from_dir` to `to`, similar to Node's `path.relative`.
/// Returns `None` if the paths cannot be compared (e.g. different path roots on Windows).
fn diff_paths(to: &Path, from_dir: &Path) -> Option<PathBuf> {
  let to = to.components().collect::<Vec<_>>();
  let from = from_dir.components().collect::<Vec<_>>();

  // Find common prefix length.
  let common = to
    .iter()
    .zip(from.iter())
    .take_while(|(a, b)| a == b)
    .count();

  let up_count = from.len() - common;
  let mut result = PathBuf::new();
  for _ in 0..up_count {
    result.push("..");
  }
  for component in &to[common..] {
    match component {
      Component::Normal(s) => result.push(s),
      Component::CurDir => {}
      Component::ParentDir => result.push(".."),
      _ => return None,
    }
  }
  Some(result)
}

/// Converts a `PathBuf` to a forward-slash URL string.
fn path_to_url_string(path: &Path) -> String {
  path
    .components()
    .filter_map(|c| match c {
      Component::Normal(s) => s.to_str().map(|s| s.to_string()),
      Component::ParentDir => Some("..".to_string()),
      Component::CurDir => Some(".".to_string()),
      _ => None,
    })
    .collect::<Vec<_>>()
    .join("/")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::Arc;

  use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
  use atlaspack_core::database::{DatabaseRef, InMemoryDatabase};
  use atlaspack_core::types::{
    Asset, Bundle, BundleBehavior, Dependency, DependencyBuilder, Environment, FileType, Priority,
    SpecifierType, Target,
  };
  use pretty_assertions::assert_eq;

  use super::replace_url_references;

  // -------------------------------------------------------------------------
  // Minimal mock BundleGraph
  // -------------------------------------------------------------------------

  struct MockBundleGraph {
    /// All bundles visible to this graph.
    bundles: Vec<Bundle>,
    /// bundle_id → assets contained in that bundle.
    assets_by_bundle: HashMap<String, Vec<Asset>>,
    /// asset_id → outgoing dependencies.
    deps_by_asset: HashMap<String, Vec<Dependency>>,
    /// lookup key (placeholder or specifier) → resolved Asset.
    resolved: HashMap<String, Asset>,
  }

  impl MockBundleGraph {
    fn new() -> Self {
      Self {
        bundles: Vec::new(),
        assets_by_bundle: HashMap::new(),
        deps_by_asset: HashMap::new(),
        resolved: HashMap::new(),
      }
    }
  }

  impl BundleGraph for MockBundleGraph {
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
      // Mirror the lookup key logic: prefer placeholder, fall back to specifier.
      let key = dependency
        .placeholder
        .as_deref()
        .unwrap_or(dependency.specifier.as_str());
      Ok(self.resolved.get(key))
    }

    fn is_dependency_skipped(&self, _dependency: &Dependency) -> bool {
      false
    }

    fn get_incoming_dependencies(&self, _asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
      Ok(vec![])
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
  }

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  fn make_db() -> DatabaseRef {
    Arc::new(InMemoryDatabase::default()) as DatabaseRef
  }

  /// Creates a CSS bundle whose output file is `dist/styles.css`.
  fn make_css_bundle(id: &str) -> Bundle {
    Bundle {
      id: id.to_string(),
      bundle_type: FileType::Css,
      entry_asset_ids: vec![],
      env: Environment::default(),
      hash_reference: String::new(),
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: Some("styles.css".to_string()),
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      bundle_behavior: None,
      is_placeholder: false,
      target: Target {
        dist_dir: PathBuf::from("/dist"),
        ..Target::default()
      },
    }
  }

  /// Creates an image asset with the given id, file type, and optional bundle_behavior.
  fn make_image_asset(id: &str, file_type: FileType, behavior: Option<BundleBehavior>) -> Asset {
    Asset {
      id: id.to_string(),
      file_type,
      bundle_behavior: behavior,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    }
  }

  /// Creates a URL dependency (the kind emitted by lightningcss for `url()` references).
  fn make_url_dep(specifier: &str, placeholder: Option<&str>) -> Dependency {
    let mut dep = DependencyBuilder::default()
      .specifier(specifier.to_string())
      .specifier_type(SpecifierType::Url)
      .priority(Priority::Sync)
      .env(Arc::new(Environment::default()))
      .build();
    dep.placeholder = placeholder.map(|s| s.to_string());
    dep
  }

  /// Creates a CSS @import dependency (is_css_import = true).
  fn make_import_dep(specifier: &str, placeholder: Option<&str>) -> Dependency {
    let mut dep = DependencyBuilder::default()
      .specifier(specifier.to_string())
      .specifier_type(SpecifierType::Url)
      .priority(Priority::Sync)
      .env(Arc::new(Environment::default()))
      .build();
    dep.placeholder = placeholder.map(|s| s.to_string());
    dep.is_css_import = true;
    dep
  }

  /// Creates a bundle that owns the given asset, with a resolved output name/path.
  fn make_image_bundle(id: &str, asset_id: &str, name: &str, dist_dir: &str) -> Bundle {
    Bundle {
      id: id.to_string(),
      bundle_type: FileType::Png,
      entry_asset_ids: vec![asset_id.to_string()],
      env: Environment::default(),
      hash_reference: String::new(),
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: Some(name.to_string()),
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      bundle_behavior: None,
      is_placeholder: false,
      target: Target {
        dist_dir: PathBuf::from(dist_dir),
        ..Target::default()
      },
    }
  }

  // -------------------------------------------------------------------------
  // Test 1: no_url_refs — CSS with no placeholder tokens is passed through unchanged
  // -------------------------------------------------------------------------

  #[test]
  fn no_url_refs_passes_css_through_unchanged() {
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let graph = MockBundleGraph::new();
    let output_dir = PathBuf::from("/dist");

    let input = "body { color: red; }\n.foo { margin: 0; }";
    let result = replace_url_references(input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert_eq!(result, input);
  }

  // -------------------------------------------------------------------------
  // Test 2: single_url_ref — one placeholder replaced with relative path to image
  // -------------------------------------------------------------------------

  #[test]
  fn single_url_ref_replaced_with_relative_path() {
    // The CSS bundle lives at /dist/styles.css.
    // The image bundle lives at /dist/images/hero.png.
    // Expected relative URL from /dist/ to /dist/images/hero.png = "images/hero.png".

    let placeholder = "abc1234567890def";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();

    let image_asset = make_image_asset("asset_img_1", FileType::Png, None);
    let image_bundle = make_image_bundle("bundle_img", "asset_img_1", "images/hero.png", "/dist");

    let url_dep = make_url_dep("./images/hero.png", Some(placeholder));

    // asset_css has the URL dep; image_asset is the resolved target
    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph.bundles.push(image_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .assets_by_bundle
      .insert("bundle_img".to_string(), vec![image_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![url_dep]);
    graph.resolved.insert(placeholder.to_string(), image_asset);

    let input = format!(".hero {{ background: url({placeholder}); }}");
    let output_dir = PathBuf::from("/dist");

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains("images/hero.png"),
      "Expected relative path 'images/hero.png' in output, got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder token must be removed from output, got: {result:?}"
    );
  }

  // -------------------------------------------------------------------------
  // Test 3: multiple_url_refs — two different placeholders both replaced
  // -------------------------------------------------------------------------

  #[test]
  fn multiple_url_refs_all_replaced_with_correct_paths() {
    let placeholder1 = "aaa1111111111111";
    let placeholder2 = "bbb2222222222222";

    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let img1 = make_image_asset("asset_img_1", FileType::Png, None);
    let img2 = make_image_asset("asset_img_2", FileType::Gif, None);
    let bundle_img1 = make_image_bundle("bundle_img_1", "asset_img_1", "images/hero.png", "/dist");
    let bundle_img2 = make_image_bundle("bundle_img_2", "asset_img_2", "images/logo.gif", "/dist");

    let dep1 = make_url_dep("./images/hero.png", Some(placeholder1));
    let dep2 = make_url_dep("./images/logo.gif", Some(placeholder2));

    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph.bundles.push(bundle_img1);
    graph.bundles.push(bundle_img2);
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .assets_by_bundle
      .insert("bundle_img_1".to_string(), vec![img1.clone()]);
    graph
      .assets_by_bundle
      .insert("bundle_img_2".to_string(), vec![img2.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep1, dep2]);
    graph.resolved.insert(placeholder1.to_string(), img1);
    graph.resolved.insert(placeholder2.to_string(), img2);

    let input = format!(
      ".a {{ background: url({placeholder1}); }} .b {{ background: url({placeholder2}); }}"
    );

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains("images/hero.png"),
      "Expected 'images/hero.png' in output, got: {result:?}"
    );
    assert!(
      result.contains("images/logo.gif"),
      "Expected 'images/logo.gif' in output, got: {result:?}"
    );
    assert!(
      !result.contains(placeholder1),
      "Placeholder1 must be removed, got: {result:?}"
    );
    assert!(
      !result.contains(placeholder2),
      "Placeholder2 must be removed, got: {result:?}"
    );
  }

  // -------------------------------------------------------------------------
  // Test 4: inline_data_uri — inline asset replaced with data URI
  // -------------------------------------------------------------------------

  #[test]
  fn inline_asset_replaced_with_base64_data_uri() {
    let placeholder = "ccc3333333333333";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    // PNG bytes stored in the DB under the asset's id
    let fake_png_bytes: &[u8] = b"\x89PNG\r\n\x1a\n";
    db.put("asset_img_inline", fake_png_bytes).unwrap();

    let inline_asset = Asset {
      id: "asset_img_inline".to_string(),
      file_type: FileType::Png,
      bundle_behavior: Some(BundleBehavior::Inline),
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let dep = make_url_dep("./inline.png", Some(placeholder));

    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    graph.resolved.insert(placeholder.to_string(), inline_asset);

    let input = format!(".icon {{ background: url({placeholder}); }}");

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains("data:image/png;base64,"),
      "Expected data URI with 'data:image/png;base64,' in output, got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder must be removed from output, got: {result:?}"
    );
  }

  // -------------------------------------------------------------------------
  // Test 5: unresolvable_url — no resolved asset means fallback to dep.specifier
  // -------------------------------------------------------------------------

  #[test]
  fn unresolvable_url_falls_back_to_specifier() {
    let placeholder = "ddd4444444444444";
    let specifier = "./missing-image.png";

    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let dep = make_url_dep(specifier, Some(placeholder));

    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    // No entry in graph.resolved -> unresolvable

    let input = format!(".missing {{ background: url({placeholder}); }}");

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains(specifier),
      "Expected original specifier '{specifier}' in fallback output, got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder must be replaced even on fallback, got: {result:?}"
    );
  }

  // -------------------------------------------------------------------------
  // Test 6: no_op_import_dep — CSS @import deps (is_css_import=true) are skipped
  // -------------------------------------------------------------------------

  #[test]
  fn css_import_dep_placeholders_are_not_processed() {
    // A dep with is_css_import = true should be ignored entirely by replace_url_references.
    // The placeholder should remain unchanged in the output (it won't appear in real CSS
    // output anyway, but if it does, the function must leave it alone).
    let import_placeholder = "eee5555555555555";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    // Even if the dep could theoretically resolve, it must be skipped because is_css_import=true
    let import_dep = make_import_dep("other.css", Some(import_placeholder));

    let image_asset = make_image_asset("asset_img_1", FileType::Png, None);
    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![import_dep]);
    // Would resolve if processed — but it must NOT be processed
    graph
      .resolved
      .insert(import_placeholder.to_string(), image_asset);

    // CSS that happens to contain the import placeholder token
    let input = format!(".rule {{ color: red; }} /* token: {import_placeholder} */");

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    // The import placeholder must NOT have been replaced with a path —
    // the token should still be present as-is because the dep was skipped.
    assert!(
      result.contains(import_placeholder),
      "CSS @import dep placeholder must not be processed/replaced, got: {result:?}"
    );
  }

  // -------------------------------------------------------------------------
  // Test 7: duplicate_placeholder — same token appearing multiple times is fully replaced
  // -------------------------------------------------------------------------

  #[test]
  fn duplicate_placeholder_all_occurrences_replaced() {
    let placeholder = "fff6666666666666";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let image_asset = make_image_asset("asset_img_1", FileType::Png, None);
    let image_bundle = make_image_bundle("bundle_img", "asset_img_1", "images/bg.png", "/dist");

    let dep = make_url_dep("./images/bg.png", Some(placeholder));

    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph.bundles.push(image_bundle);
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .assets_by_bundle
      .insert("bundle_img".to_string(), vec![image_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    graph.resolved.insert(placeholder.to_string(), image_asset);

    // The same placeholder appears three times (e.g. repeated background declarations)
    let input = format!(
      ".a {{ background: url({placeholder}); }} .b {{ background: url({placeholder}); }} .c {{ background: url({placeholder}); }}"
    );

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      !result.contains(placeholder),
      "All occurrences of placeholder must be replaced, got: {result:?}"
    );
    // All three occurrences should be the resolved path
    let path_occurrences: Vec<_> = result.matches("images/bg.png").collect();
    assert_eq!(
      path_occurrences.len(),
      3,
      "Expected exactly 3 replacements of the placeholder, got: {result:?}"
    );
  }

  #[test]
  fn inline_svg_asset_replaced_with_correct_mime_type() {
    let placeholder = "svg_placeholder";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");
    let svg_content = "<svg>...</svg>";
    // Fix: Pass slice instead of Vec
    db.put("svg_content", svg_content.as_bytes()).unwrap();

    // Create an inline SVG asset
    let svg_asset = Asset {
      id: "asset_svg_1".to_string(),
      file_type: FileType::Other("svg".to_string()),
      env: Arc::new(Environment::default()),
      bundle_behavior: Some(BundleBehavior::Inline),
      content_key: Some("svg_content".to_string()),
      ..Asset::default()
    };

    let dep = make_url_dep("icon.svg", Some(placeholder));

    let css_asset = Asset {
      id: "asset_css_1".to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    graph.resolved.insert(placeholder.to_string(), svg_asset);

    let input = format!(".icon {{ background: url({}); }}", placeholder);

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(svg_content);
    let expected = format!(
      ".icon {{ background: url(data:image/svg+xml;base64,{}); }}",
      encoded
    );

    assert_eq!(
      result, expected,
      "SVG data URI should have correct MIME type"
    );
  }
}
