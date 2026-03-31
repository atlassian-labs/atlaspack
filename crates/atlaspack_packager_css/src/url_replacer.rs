use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

/// Mirrors JS `fixedEncodeURIComponent`: encode everything except `A-Z a-z 0-9 - _ . ~`.
///
/// Standard `encodeURIComponent` leaves `- _ . ! ~ * ' ( )` unencoded; `fixedEncodeURIComponent`
/// additionally encodes `! * ' ( )`, leaving only the four RFC-3986 unreserved non-alphanum chars.
const FIXED_ENCODE_URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
  .remove(b'-')
  .remove(b'_')
  .remove(b'.')
  .remove(b'~');

use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
use atlaspack_core::database::DatabaseRef;
use atlaspack_core::types::{Asset, Bundle, BundleBehavior, FileType};

/// Replaces `url()` placeholder tokens with resolved relative paths or data URIs.
pub fn replace_url_references(
  css: &str,
  bundle: &Bundle,
  bundle_graph: &dyn BundleGraph,
  db: &DatabaseRef,
  output_dir: &Path,
) -> anyhow::Result<String> {
  let bundle_assets = bundle_graph.get_bundle_assets(bundle)?;

  // Collect all non-CSS-import URL dependencies across all assets in the bundle.
  let mut url_deps = Vec::new();
  for asset in &bundle_assets {
    for dep in bundle_graph.get_dependencies(asset)? {
      if dep.is_css_import {
        continue;
      }
      let token = dep.placeholder.as_deref().unwrap_or(dep.specifier.as_str());
      let resolved = bundle_graph.get_resolved_asset(dep, bundle)?;
      url_deps.push((token.to_string(), dep, resolved));
    }
  }

  if url_deps.is_empty() {
    return Ok(css.to_string());
  }

  let target_asset_ids: HashSet<&str> = url_deps
    .iter()
    .filter_map(|(_, _, resolved)| resolved.map(|a| a.id.as_str()))
    .collect();

  let assets_to_bundle = find_assets_to_bundle(bundle_graph, target_asset_ids);

  // Map each token to its replacement string.
  let mut replacement_map = HashMap::new();

  for (token, dep, resolved) in &url_deps {
    let token = token.as_str();
    let placeholder_exists = css.contains(token) && !replacement_map.contains_key(token);
    if !placeholder_exists {
      continue;
    }

    // Extract any URL fragment (e.g. `sprite.svg#icon`) from the specifier.
    let (specifier_base, fragment) = dep
      .specifier
      .split_once('#')
      .map(|(base, frag)| (base, Some(frag)))
      .unwrap_or((dep.specifier.as_str(), None));

    let replacement = match *resolved {
      None => append_fragment(escape_css_string(specifier_base), fragment),
      Some(asset) => {
        if is_inline_behavior(asset.bundle_behavior) {
          to_data_uri(asset, db)?
        } else {
          let resolved_path = assets_to_bundle
            .get(asset.id.as_str())
            .and_then(|target_bundle| {
              find_relative_path_for_bundle(target_bundle, bundle, output_dir)
            })
            .or_else(|| {
              // Fallback for assets already in the current bundle
              bundle_assets
                .iter()
                .find(|a| a.id == asset.id)
                .map(|_| specifier_base.to_string())
            });

          match resolved_path {
            Some(path) => append_fragment(escape_css_string(&path), fragment),
            None => append_fragment(escape_css_string(specifier_base), fragment),
          }
        }
      }
    };

    replacement_map.insert(token, replacement);
  }

  if replacement_map.is_empty() {
    return Ok(css.to_string());
  }

  let mut result = css.to_string();
  for (token, replacement) in &replacement_map {
    result = result.replace(token, replacement);
  }
  Ok(result)
}

/// Pre-calculates asset-to-non-inline-bundle mapping for required assets.
fn find_assets_to_bundle<'a>(
  bundle_graph: &'a dyn BundleGraph,
  target_asset_ids: HashSet<&'a str>,
) -> HashMap<&'a str, &'a Bundle> {
  let mut assets_to_bundle = HashMap::new();

  if !target_asset_ids.is_empty() {
    for bundle in bundle_graph.get_bundles() {
      if is_inline_behavior(bundle.bundle_behavior) {
        continue;
      }

      if let Ok(assets) = bundle_graph.get_bundle_assets(bundle) {
        for asset in assets {
          if target_asset_ids.contains(asset.id.as_str()) {
            assets_to_bundle.insert(asset.id.as_str(), bundle);
          }
        }
      }
    }
  }

  assets_to_bundle
}

fn is_inline_behavior(behavior: Option<BundleBehavior>) -> bool {
  matches!(
    behavior,
    Some(BundleBehavior::Inline) | Some(BundleBehavior::InlineIsolated)
  )
}

/// Appends a URL fragment to a base URL string when one is present.
fn append_fragment(base: String, fragment: Option<&str>) -> String {
  match fragment {
    Some(f) => format!("{base}#{f}"),
    None => base,
  }
}

/// Converts an asset's content to a data URI, either percent-encoded (for text/SVG)
/// or base64-encoded (for binary).
fn to_data_uri(asset: &Asset, db: &DatabaseRef) -> anyhow::Result<String> {
  let db_key = asset.content_key.as_deref().unwrap_or(asset.id.as_str());
  let bytes = db.get(db_key)?.unwrap_or_default();
  let mime = mime_for_file_type(&asset.file_type);

  // Mirror @atlaspack/optimizer-data-url: text (valid UTF-8) → percent-encode,
  // binary → base64. SVG and other text formats are percent-encoded; PNG/WebP/etc
  // are base64-encoded.
  match std::str::from_utf8(&bytes) {
    Ok(text) => {
      let encoded = utf8_percent_encode(text, FIXED_ENCODE_URI_COMPONENT);
      Ok(format!("data:{mime},{encoded}"))
    }
    Err(_) => {
      let encoded = BASE64_STANDARD.encode(&bytes);
      Ok(format!("data:{mime};base64,{encoded}"))
    }
  }
}

/// Escapes `"` and `\` in a CSS string value to prevent breaking string syntax.
/// Mirrors `escapeString` from `CSSPackager.ts`.
fn escape_css_string(s: &str) -> String {
  s.replace('\\', "\\\\").replace('"', "\\\"")
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
    // All other file types (e.g. Js, Css, Html) are treated as binary blobs.
    _ => "application/octet-stream",
  }
}

/// Resolves a forward-slash relative path from `css_bundle` to `target_bundle`.
fn find_relative_path_for_bundle(
  target_bundle: &Bundle,
  css_bundle: &Bundle,
  output_dir: &Path,
) -> Option<String> {
  let to_name = target_bundle.name.as_deref().filter(|n| !n.is_empty())?;

  let from_name = css_bundle.name.as_deref().unwrap_or("");
  let from_dir_buf = output_dir.join(from_name);
  let from_dir = from_dir_buf.parent().unwrap_or(output_dir);

  let to_file = target_bundle.target.dist_dir.join(to_name);
  let rel = pathdiff::diff_paths(&to_file, from_dir)?;
  Some(path_to_url_string(&rel))
}

pub(crate) fn path_to_url_string(path: &Path) -> String {
  path
    .components()
    .filter_map(|c| match c {
      Component::Normal(s) => s.to_str(),
      Component::ParentDir => Some(".."),
      Component::CurDir => Some("."),
      _ => None,
    })
    .fold(String::new(), |mut acc, part| {
      if !acc.is_empty() {
        acc.push('/');
      }
      acc.push_str(part);
      acc
    })
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::Arc;

  use super::escape_css_string;
  use atlaspack_core::bundle_graph::bundle_graph::BundleGraph;
  use atlaspack_core::database::{DatabaseRef, InMemoryDatabase};
  use atlaspack_core::types::{
    Asset, Bundle, BundleBehavior, Dependency, DependencyBuilder, Environment, FileType, Priority,
    SpecifierType, Target,
  };
  use pretty_assertions::assert_eq;

  use super::{append_fragment, mime_for_file_type, path_to_url_string, replace_url_references};

  struct MockBundleGraph {
    bundles: Vec<Bundle>,
    assets_by_bundle: HashMap<String, Vec<Asset>>,
    deps_by_asset: HashMap<String, Vec<Dependency>>,
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

  fn make_db() -> DatabaseRef {
    Arc::new(InMemoryDatabase::default()) as DatabaseRef
  }

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

  fn make_image_asset(id: &str, file_type: FileType, behavior: Option<BundleBehavior>) -> Asset {
    Asset {
      id: id.to_string(),
      file_type,
      bundle_behavior: behavior,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    }
  }

  fn make_css_asset(id: &str) -> Asset {
    Asset {
      id: id.to_string(),
      file_type: FileType::Css,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    }
  }

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

  fn make_import_dep(specifier: &str, placeholder: Option<&str>) -> Dependency {
    let mut dep = make_url_dep(specifier, placeholder);
    dep.is_css_import = true;
    dep
  }

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
    let css_asset = make_css_asset("asset_css_1");

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

    let css_asset = make_css_asset("asset_css_1");

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

  #[test]
  fn css_import_dep_placeholders_are_not_processed() {
    let import_placeholder = "eee5555555555555";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let import_dep = make_import_dep("other.css", Some(import_placeholder));

    let image_asset = make_image_asset("asset_img_1", FileType::Png, None);
    let css_asset = make_css_asset("asset_css_1");

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![import_dep]);
    graph
      .resolved
      .insert(import_placeholder.to_string(), image_asset);

    let input = format!(".rule {{ color: red; }} /* token: {import_placeholder} */");

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains(import_placeholder),
      "CSS @import dep placeholder must not be processed/replaced, got: {result:?}"
    );
  }

  #[test]
  fn duplicate_placeholder_all_occurrences_replaced() {
    let placeholder = "fff6666666666666";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let image_asset = make_image_asset("asset_img_1", FileType::Png, None);
    let image_bundle = make_image_bundle("bundle_img", "asset_img_1", "images/bg.png", "/dist");

    let dep = make_url_dep("./images/bg.png", Some(placeholder));

    let css_asset = make_css_asset("asset_css_1");

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

    let input = format!(
      ".a {{ background: url({placeholder}); }} .b {{ background: url({placeholder}); }} .c {{ background: url({placeholder}); }}"
    );

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      !result.contains(placeholder),
      "All occurrences of placeholder must be replaced, got: {result:?}"
    );
    let path_occurrences: Vec<_> = result.matches("images/bg.png").collect();
    assert_eq!(
      path_occurrences.len(),
      3,
      "Expected exactly 3 replacements of the placeholder, got: {result:?}"
    );
  }

  #[test]
  fn inline_svg_asset_replaced_with_percent_encoded_data_uri() {
    // SVG is valid UTF-8, so it must be percent-encoded (not base64), matching
    // @atlaspack/optimizer-data-url behaviour.
    let placeholder = "svg_placeholder";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");
    let svg_content =
      "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"8\" height=\"8\"/></svg>";
    db.put("svg_content", svg_content.as_bytes()).unwrap();

    let svg_asset = Asset {
      id: "asset_svg_1".to_string(),
      file_type: FileType::Other("svg".to_string()),
      env: Arc::new(Environment::default()),
      bundle_behavior: Some(BundleBehavior::Inline),
      content_key: Some("svg_content".to_string()),
      ..Asset::default()
    };

    let dep = make_url_dep("icon.svg", Some(placeholder));

    let css_asset = make_css_asset("asset_css_1");

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

    // Must be percent-encoded, not base64.
    assert!(
      result.contains("data:image/svg+xml,%3Csvg"),
      "SVG data URI must be percent-encoded; got: {result:?}"
    );
    assert!(
      !result.contains("base64"),
      "SVG (text) must not be base64-encoded; got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder must be replaced; got: {result:?}"
    );
  }

  #[test]
  fn inline_binary_asset_replaced_with_base64_data_uri() {
    // Binary content (not valid UTF-8) must be base64-encoded, matching
    // @atlaspack/optimizer-data-url behaviour.
    let placeholder = "ccc3333333333333";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

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

    let css_asset = make_css_asset("asset_css_1");

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
      "Binary PNG must be base64-encoded; got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder must be removed from output, got: {result:?}"
    );
  }

  #[test]
  fn escape_css_string_escapes_quotes_and_backslashes() {
    assert_eq!(escape_css_string("normal/path.svg"), "normal/path.svg");
    assert_eq!(escape_css_string("path/with\"quote"), "path/with\\\"quote");
    assert_eq!(
      escape_css_string("path\\with\\backslash"),
      "path\\\\with\\\\backslash"
    );
    assert_eq!(
      escape_css_string("both\"and\\mixed"),
      "both\\\"and\\\\mixed"
    );
  }

  #[test]
  fn append_fragment_appends_hash_or_returns_base_unchanged() {
    // With a fragment
    assert_eq!(
      append_fragment("path/to/file.svg".to_string(), Some("icon")),
      "path/to/file.svg#icon"
    );
    assert_eq!(
      append_fragment("file.png".to_string(), Some("section")),
      "file.png#section"
    );
    // Without a fragment
    assert_eq!(
      append_fragment("path/to/file.svg".to_string(), None),
      "path/to/file.svg"
    );
    assert_eq!(append_fragment(String::new(), None), "");
  }

  #[test]
  fn inline_bundle_assets_are_not_candidates_for_path_resolution() {
    // Assets whose containing bundle has Inline or InlineIsolated behavior must not be
    // resolved to a relative path — they should be inlined as a data URI instead.
    // This exercises the is_inline_behavior guard inside find_assets_to_bundle.
    let placeholder = "ddd4444444444444";
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    // The image asset itself is *not* inline, but its bundle is.
    // replace_url_references should not find it via path resolution and fall back
    // to the specifier.
    let image_asset = make_image_asset("asset_img_inline_bundle", FileType::Png, None);
    let mut inline_bundle = make_image_bundle(
      "bundle_img_inline",
      "asset_img_inline_bundle",
      "images/hero.png",
      "/dist",
    );
    inline_bundle.bundle_behavior = Some(BundleBehavior::Inline);

    let css_bundle = make_css_bundle("bundle_css");
    let css_asset = make_css_asset("asset_css_1");
    let dep = make_url_dep("./images/hero.png", Some(placeholder));

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph.bundles.push(inline_bundle);
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset]);
    graph
      .assets_by_bundle
      .insert("bundle_img_inline".to_string(), vec![image_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    graph.resolved.insert(placeholder.to_string(), image_asset);

    let input = format!(".hero {{ background: url({placeholder}); }}");
    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    // The inline bundle is excluded from path resolution so no relative path is computed
    // via bundle lookup; the output falls back to the original specifier string.
    assert!(
      !result.contains(placeholder),
      "Placeholder must be replaced; got: {result:?}"
    );
    assert!(
      result.contains("./images/hero.png"),
      "Should fall back to the original specifier when the owning bundle is inline; got: {result:?}"
    );
  }

  #[test]
  fn mime_for_file_type_known_image_types() {
    assert_eq!(mime_for_file_type(&FileType::Png), "image/png");
    assert_eq!(mime_for_file_type(&FileType::Jpeg), "image/jpeg");
    assert_eq!(mime_for_file_type(&FileType::Gif), "image/gif");
    assert_eq!(mime_for_file_type(&FileType::WebP), "image/webp");
    assert_eq!(mime_for_file_type(&FileType::Avif), "image/avif");
    assert_eq!(mime_for_file_type(&FileType::Tiff), "image/tiff");
  }

  #[test]
  fn mime_for_file_type_other_image_and_font_extensions() {
    assert_eq!(
      mime_for_file_type(&FileType::Other("svg".to_string())),
      "image/svg+xml"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Other("woff2".to_string())),
      "font/woff2"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Other("woff".to_string())),
      "font/woff"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Other("ttf".to_string())),
      "font/ttf"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Other("eot".to_string())),
      "application/vnd.ms-fontobject"
    );
  }

  #[test]
  fn mime_for_file_type_unrecognised_types_return_octet_stream() {
    // Unknown Other extensions
    assert_eq!(
      mime_for_file_type(&FileType::Other("bin".to_string())),
      "application/octet-stream"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Other("xyz".to_string())),
      "application/octet-stream"
    );
    // Non-image typed assets (Js, Css, Html, etc.)
    assert_eq!(
      mime_for_file_type(&FileType::Js),
      "application/octet-stream"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Css),
      "application/octet-stream"
    );
    assert_eq!(
      mime_for_file_type(&FileType::Html),
      "application/octet-stream"
    );
  }

  #[test]
  fn path_to_url_string_converts_path_components() {
    // Simple relative path
    assert_eq!(
      path_to_url_string(std::path::Path::new("images/hero.png")),
      "images/hero.png"
    );
    assert_eq!(
      path_to_url_string(std::path::Path::new("file.css")),
      "file.css"
    );
    // Parent-dir traversal
    assert_eq!(
      path_to_url_string(std::path::Path::new("../images/hero.png")),
      "../images/hero.png"
    );
    assert_eq!(
      path_to_url_string(std::path::Path::new("../../fonts/fira.woff2")),
      "../../fonts/fira.woff2"
    );
    // Current-dir component
    assert_eq!(
      path_to_url_string(std::path::Path::new("./images/hero.png")),
      "./images/hero.png"
    );
    // Root prefix is stripped (Unix: Prefix/RootDir component is dropped)
    assert_eq!(
      path_to_url_string(std::path::Path::new("/images/hero.png")),
      "images/hero.png"
    );
  }

  #[test]
  fn unresolvable_url_with_fragment_preserves_fragment_in_fallback() {
    let placeholder = "frag_placeholder";
    let specifier = "./sprite.svg#icon";

    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let dep = make_url_dep(specifier, Some(placeholder));

    let css_asset = make_css_asset("asset_css_1");

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    // No resolved asset → falls back to specifier.

    let input = format!(".icon {{ background: url({placeholder}); }}");
    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains("./sprite.svg"),
      "Base specifier path must be present in fallback; got: {result:?}"
    );
    assert!(
      result.contains("#icon"),
      "Fragment must be preserved in unresolvable-URL fallback; got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder must be replaced; got: {result:?}"
    );
  }

  #[test]
  fn inline_isolated_asset_replaced_with_data_uri() {
    let placeholder = "isolated_placeholder";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let png_bytes: &[u8] = b"\x89PNG\r\n";
    db.put("isolated_img", png_bytes).unwrap();

    let isolated_asset = Asset {
      id: "isolated_img".to_string(),
      file_type: FileType::Png,
      bundle_behavior: Some(BundleBehavior::InlineIsolated),
      content_key: Some("isolated_img".to_string()),
      env: Arc::new(Environment::default()),
      ..Asset::default()
    };

    let dep = make_url_dep("./isolated.png", Some(placeholder));
    let css_asset = make_css_asset("asset_css_isolated");

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset]);
    graph
      .deps_by_asset
      .insert("asset_css_isolated".to_string(), vec![dep]);
    graph
      .resolved
      .insert(placeholder.to_string(), isolated_asset);

    let input = format!(".img {{ background: url({placeholder}); }}");
    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains("data:image/png;base64,"),
      "InlineIsolated asset must produce a data URI; got: {result:?}"
    );
  }

  #[test]
  fn unresolvable_url_exact_output() {
    let placeholder = "exact_placeholder";
    let specifier = "./missing.png";

    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let dep = make_url_dep(specifier, Some(placeholder));
    let css_asset = make_css_asset("asset_css_1");

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);

    let input = format!(".rule {{ background: url({placeholder}); }}");
    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert_eq!(result, ".rule { background: url(./missing.png); }");
  }

  #[test]
  fn url_replacement_preserves_hash_fragment() {
    let placeholder = "svg_sprite_placeholder";
    let css_bundle = make_css_bundle("bundle_css");
    let db = make_db();
    let output_dir = PathBuf::from("/dist");

    let image_asset = make_image_asset("asset_sprite_1", FileType::Other("svg".to_string()), None);
    let image_bundle = make_image_bundle(
      "bundle_sprite",
      "asset_sprite_1",
      "images/sprite.svg",
      "/dist",
    );

    // Specifier includes a hash fragment.
    let dep = make_url_dep("./images/sprite.svg#icon", Some(placeholder));

    let css_asset = make_css_asset("asset_css_1");

    let mut graph = MockBundleGraph::new();
    graph.bundles.push(css_bundle.clone());
    graph.bundles.push(image_bundle);
    graph
      .assets_by_bundle
      .insert("bundle_css".to_string(), vec![css_asset.clone()]);
    graph
      .assets_by_bundle
      .insert("bundle_sprite".to_string(), vec![image_asset.clone()]);
    graph
      .deps_by_asset
      .insert("asset_css_1".to_string(), vec![dep]);
    graph.resolved.insert(placeholder.to_string(), image_asset);

    let input = format!(".icon {{ background: url({placeholder}); }}");

    let result = replace_url_references(&input, &css_bundle, &graph, &db, &output_dir)
      .expect("replace_url_references must succeed");

    assert!(
      result.contains("#icon"),
      "Hash fragment must be preserved in output; got: {result:?}"
    );
    assert!(
      !result.contains(placeholder),
      "Placeholder must be replaced; got: {result:?}"
    );
  }
}
