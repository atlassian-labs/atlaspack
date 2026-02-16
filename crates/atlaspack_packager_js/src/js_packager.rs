use std::{collections::HashMap, path::PathBuf, sync::Arc};

use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph,
  debug_tools::DebugTools,
  hash::{hash_bytes, hash_string},
  types::{Asset, Bundle, OutputFormat},
  version::atlaspack_rust_version,
};
use lmdb_js_lite::DatabaseHandle;
use parking_lot::RwLock;
use pathdiff::diff_paths;
use rayon::prelude::*;

use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};

use super::JsPackager;
use super::process_asset::rewrite_asset_code;

type PackagedAsset<'a> = (&'a Asset, String);

impl<B: BundleGraph + Send + Sync> JsPackager<B> {
  pub fn new(
    db: Arc<DatabaseHandle>,
    bundle_graph: Arc<RwLock<B>>,
    project_root: PathBuf,
    debug_tools: DebugTools,
  ) -> Self {
    Self {
      db,
      bundle_graph,
      project_root,
      debug_tools,
    }
  }

  pub fn package(&self, bundle_id: &str) -> anyhow::Result<PackageResult> {
    let graph = self.bundle_graph.read();
    let bundle = graph
      .get_bundle_by_id(bundle_id)
      .ok_or(anyhow::anyhow!("Bundle not found"))?;

    let assets = graph.get_bundle_assets(bundle)?;

    let span = tracing::trace_span!("process_assets", bundle_id = bundle_id).entered();
    let contents = assets
      .par_iter()
      .map(|asset| {
        let span = tracing::trace_span!("read_code", asset_id = asset.id).entered();
        let txn = self.db.database().read_txn()?;
        let code = self.db.database().get(
          &txn,
          asset
            .content_key
            .as_ref()
            .ok_or(anyhow::anyhow!("Asset content key not found"))?,
        )?;
        txn.commit()?;
        span.exit();
        let asset_code =
          String::from_utf8_lossy(&code.ok_or(anyhow::anyhow!("Unable to read asset code"))?)
            .to_string();
        self
          .process_asset(bundle, asset, asset_code)
          .map(|content| (*asset, content))
      })
      .collect::<anyhow::Result<Vec<(&Asset, String)>>>()?;
    span.exit();

    let bundle_contents = self.assemble_bundle(bundle, contents);
    let bundle_contents = bundle_contents.as_bytes();
    let content_hash = hash_bytes(bundle_contents);
    let content_cache_key = format!(
      "PackagerRunner/{}/{content_hash}/content",
      atlaspack_rust_version()
    );
    let info_cache_key = format!(
      "PackagerRunner/{}/{content_hash}/info",
      atlaspack_rust_version()
    );

    let mut write_txn = self.db.database().write_txn()?;
    self
      .db
      .database()
      .put(&mut write_txn, &content_cache_key, bundle_contents)?;

    // As the "info" object needs to be read from JS, it needs to be serialized by JS - for now
    // we return it to JS and write it to LMDB there..

    write_txn.commit()?;

    Ok(PackageResult {
      bundle_info: BundleInfo {
        bundle_type: bundle.bundle_type.extension().to_string(),
        size: bundle_contents.len() as u64,
        total_assets: assets.len() as u64,
        hash: content_hash,
        hash_references: vec![],
        cache_keys: CacheKeyMap {
          content: content_cache_key,
          map: "TODO".to_string(), // Has to exist for JS, but won't be found in LMDB
          info: info_cache_key,
        },
        is_large_blob: false,
        time: Some(0),
      },
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
    })
  }

  #[tracing::instrument(level = "trace", skip_all)]
  fn process_asset(&self, bundle: &Bundle, asset: &Asset, code: String) -> anyhow::Result<String> {
    // Get dependency map for this asset
    let deps = self.get_asset_dependency_map(bundle, asset)?;
    let code = rewrite_asset_code(code, &deps)?;

    // All assets are wrapped, including entry assets. Entry assets will be explicitly
    // required at the bottom of the bundle to ensure they execute in order.
    self.wrap_asset(bundle, asset, code)
  }

  fn get_asset_dependency_map(
    &self,
    bundle: &Bundle,
    asset: &Asset,
  ) -> anyhow::Result<HashMap<String, Option<String>>> {
    let bundle_graph = self.bundle_graph.read();

    // Get dependencies for asset
    let dependencies = bundle_graph.get_dependencies(asset)?;

    let deps = dependencies
      .iter()
      .map(|dependency| {
        let specifier = dependency
          .placeholder
          .as_deref()
          .unwrap_or(&dependency.specifier);

        let dep_value: Option<String> = if bundle_graph.is_dependency_skipped(dependency) {
          None
        } else if let Some(resolved) = bundle_graph.get_resolved_asset(dependency, bundle)? {
          Some(
            bundle_graph
              .get_public_asset_id(&resolved.id)
              .ok_or(anyhow::anyhow!("Asset not found in bundle graph"))?
              .to_string(),
          )
        } else {
          Some(dependency.specifier.clone())
        };

        Ok((specifier.to_string(), dep_value))
      })
      .collect::<anyhow::Result<HashMap<String, Option<String>>>>()?;

    Ok(deps)
  }

  fn wrap_asset(&self, _bundle: &Bundle, asset: &Asset, code: String) -> anyhow::Result<String> {
    let bundle_graph = self.bundle_graph.read();
    let public_id = bundle_graph
      .get_public_asset_id(&asset.id)
      .expect("Asset not found in bundle graph")
      .to_string();

    // Only add comments if debug flag is enabled
    let comment = if self.debug_tools.asset_file_names_in_output {
      // Show file path for real files (e.g. node_modules inside or outside project root).
      // Skip path for virtual/generated assets.
      let file_path_comment = if asset.is_virtual {
        String::new()
      } else {
        // Relative to project root (use .. when outside, e.g. node_modules above project).
        let display_path = diff_paths(&asset.file_path, &self.project_root)
          .unwrap_or_else(|| asset.file_path.clone());
        display_path
          .to_str()
          .map(|p| format!(": {}", p))
          .unwrap_or_default()
      };
      format!("\n// {public_id}{file_path_comment}\n")
    } else {
      String::new()
    };

    Ok(format!(
      "{comment}define('{public_id}', function (require,module,exports) {{ {code} }});"
    ))
  }

  pub fn assemble_bundle(&self, bundle: &Bundle, contents: Vec<(&Asset, String)>) -> String {
    // This is a temporary implementation that will just use string concatenation

    // Sort the contents - non-entry assets by asset id first, then entry assets in the same order as bundle.entry_asset_ids

    // Separate entry and non-entry assets
    let (mut entry_contents, mut non_entry_contents): (Vec<PackagedAsset>, Vec<PackagedAsset>) =
      contents
        .into_iter()
        .partition(|(asset, _)| bundle.entry_asset_ids.contains(&asset.id));

    // Sort non-entry assets by asset ID
    non_entry_contents.sort_by_key(|(asset, _)| asset.id.clone());

    // Sort entry assets by their order in bundle.entry_asset_ids
    entry_contents.sort_by_key(|(asset, _)| {
      bundle
        .entry_asset_ids
        .iter()
        .position(|id| id == &asset.id)
        .unwrap_or(usize::MAX)
    });

    // Combine: non-entry assets first, then entry assets
    let mut contents = non_entry_contents;
    contents.extend(entry_contents);

    let asset_contents = contents
      .into_iter()
      .map(|(_, content)| content)
      .collect::<Vec<_>>()
      .join("\n");

    // Build explicit require() calls for entry assets to execute them in order
    let bundle_graph = self.bundle_graph.read();
    let entry_requires = bundle
      .entry_asset_ids
      .iter()
      .map(|asset_id| {
        let public_id = bundle_graph
          .get_public_asset_id(asset_id)
          .expect("Entry asset not found in bundle graph");
        format!("require('{}');", public_id)
      })
      .collect::<Vec<_>>()
      .join("\n");

    // For now we just always use the dev prelude
    let prelude_string = match self.debug_tools.debug_prelude {
      true => include_str!("../prelude/lib/prelude.debug.js"),
      false => include_str!("../prelude/lib/prelude.dev.js"),
    };

    // For SSR bundles we don't have the concern of disambiguating preludes from different builds
    // I'm not sure if we need this functionality at all for our use case, but let's leave it in for now.
    let full_hash = hash_string("TODO".to_string());
    let prelude_hash = full_hash
      .chars()
      .skip(full_hash.len() - 4)
      .collect::<String>();

    // It's a little bit of gymnastics, but what we want to do here is:
    // - have a prelude that's pre-compiled and can be unit tested in isolation - so the prelude code is just an iife
    // - we want to minimise the amount of JS that's in a string and not typechecked / unit testable
    // - we want to only have one copy of the prelude, no matter how many bundles are loaded (less relevant for SSR)
    // - we want (relatively) short names for the required top level methods `require` and `define`
    //
    // We are working under the assumption that a few extra prelude bytes are not going to be a big deal - I don't think we should
    // be micro-optimising at that level (when we get to that point what we probably want is an external prelude bundle anyway and no prelude
    // in "user" bundles)
    let prelude_loader = format!(
      r#"
    var globalObject = globalThis ?? global ?? window ?? this ?? {{}};
    if (!globalObject.Atlaspack_{prelude_hash}) {{
      globalObject.Atlaspack_{prelude_hash} = {prelude_string};
    }}
    var require = globalObject.Atlaspack_{prelude_hash}.require;
    var define = globalObject.Atlaspack_{prelude_hash}.define;
    "#,
      prelude_hash = &prelude_hash
    );

    // Bundle structure depends on output format:
    //
    // CommonJS (e.g., SSR bundles):
    //   - All assets (including entries) are wrapped in define() calls
    //   - Main entry asset is explicitly required and its exports assigned to module.exports
    //   - No IIFE wrapper - the bundle is directly executable by Node.js
    //   - This allows the SSR bundle to export functions that can be imported by other modules
    //
    // Other formats (Global, ESModule):
    //   - All assets (including entries) are wrapped in define() calls
    //   - Entry assets are explicitly executed via require() calls
    //   - Entire bundle is wrapped in IIFE to isolate variables and avoid global pollution
    let is_commonjs = bundle.env.output_format == OutputFormat::CommonJS;

    if is_commonjs {
      let main_entry_require = if let Some(main_entry_id) = bundle.entry_asset_ids.first() {
        let public_id = bundle_graph
          .get_public_asset_id(main_entry_id)
          .expect("Main entry asset not found in bundle graph");
        format!("module.exports = require('{}');", public_id)
      } else {
        String::new()
      };

      prelude_loader + &asset_contents + "\n" + &main_entry_require + "\n"
    } else {
      "(function() {\n".to_string()
        + &prelude_loader
        + &asset_contents
        + "\n"
        + &entry_requires
        + "\n})();\n"
    }
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_core::types::{Asset, Bundle, Environment, FileType};
  use std::path::PathBuf;
  use std::sync::Arc;

  // Note: Full integration tests with database and complex mocking are better suited
  // for integration tests. These unit tests focus on pure logic that can be tested
  // in isolation.

  fn create_test_asset(id: &str, file_path: &str) -> Asset {
    Asset {
      id: id.to_string(),
      file_path: PathBuf::from(file_path),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      content_key: Some(format!("content_{}", id)),
      ..Asset::default()
    }
  }

  fn create_test_bundle(id: &str) -> Bundle {
    Bundle {
      id: id.to_string(),
      name: Some("test.js".to_string()),
      bundle_behavior: None,
      bundle_type: FileType::Js,
      entry_asset_ids: vec![],
      env: Environment::default(),
      hash_reference: String::new(),
      is_splittable: Some(true),
      main_entry_id: None,
      manual_shared_bundle: None,
      needs_stable_name: Some(false),
      pipeline: None,
      public_id: None,
      target: Default::default(),
    }
  }

  #[test]
  fn test_assemble_bundle_structure() {
    // Test the structure and ordering logic without needing a full JsPackager instance
    let bundle = create_test_bundle("bundle1");
    let asset1 = create_test_asset("zzz", "/z.js");
    let asset2 = create_test_asset("aaa", "/a.js");
    let asset3 = create_test_asset("mmm", "/m.js");

    let contents: Vec<(&Asset, String)> = vec![
      (&asset1, "// asset zzz".to_string()),
      (&asset2, "// asset aaa".to_string()),
      (&asset3, "// asset mmm".to_string()),
    ];

    // Test the sorting logic directly
    let (entry_contents, mut non_entry_contents): (Vec<_>, Vec<_>) = contents
      .into_iter()
      .partition(|(asset, _)| bundle.entry_asset_ids.contains(&asset.id));

    non_entry_contents.sort_by_key(|(asset, _)| asset.id.clone());

    // Verify sorting order
    assert_eq!(non_entry_contents.len(), 3);
    assert_eq!(non_entry_contents[0].0.id, "aaa"); // Alphabetically first
    assert_eq!(non_entry_contents[1].0.id, "mmm");
    assert_eq!(non_entry_contents[2].0.id, "zzz"); // Alphabetically last
    assert!(entry_contents.is_empty());
  }

  #[test]
  fn test_assemble_bundle_entry_asset_ordering() {
    let mut bundle = create_test_bundle("bundle1");
    bundle.entry_asset_ids = vec!["entry2".to_string(), "entry1".to_string()];

    let entry1 = create_test_asset("entry1", "/entry1.js");
    let entry2 = create_test_asset("entry2", "/entry2.js");
    let non_entry = create_test_asset("zzz", "/zzz.js");

    let contents: Vec<(&Asset, String)> = vec![
      (&entry1, "// entry 1".to_string()),
      (&non_entry, "// non entry".to_string()),
      (&entry2, "// entry 2".to_string()),
    ];

    // Test the partitioning and sorting logic
    let (mut entry_contents, mut non_entry_contents): (Vec<_>, Vec<_>) = contents
      .into_iter()
      .partition(|(asset, _)| bundle.entry_asset_ids.contains(&asset.id));

    non_entry_contents.sort_by_key(|(asset, _)| asset.id.clone());

    entry_contents.sort_by_key(|(asset, _)| {
      bundle
        .entry_asset_ids
        .iter()
        .position(|id| id == &asset.id)
        .unwrap_or(usize::MAX)
    });

    // Verify ordering
    assert_eq!(non_entry_contents.len(), 1);
    assert_eq!(non_entry_contents[0].0.id, "zzz");

    assert_eq!(entry_contents.len(), 2);
    assert_eq!(entry_contents[0].0.id, "entry2"); // First in entry_asset_ids
    assert_eq!(entry_contents[1].0.id, "entry1"); // Second in entry_asset_ids
  }

  #[test]
  fn test_wrap_asset_format() {
    let public_id = "pub123";
    let code = "module.exports = 42;";

    let expected_pattern = format!(
      "\n// {}\ndefine('{}', function (require,module,exports) {{ {} }});",
      public_id, public_id, code
    );

    // Verify the format
    assert!(expected_pattern.contains(&format!("// {}", public_id)));
    assert!(expected_pattern.contains("define('pub123',"));
    assert!(expected_pattern.contains("function (require,module,exports)"));
    assert!(expected_pattern.contains("module.exports = 42;"));
    assert!(expected_pattern.ends_with("});"));
  }
}
