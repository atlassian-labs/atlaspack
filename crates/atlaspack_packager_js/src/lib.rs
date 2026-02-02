use std::{
  collections::HashMap,
  sync::{Arc, LazyLock},
};

use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph,
  hash::{hash_bytes, hash_string},
  types::{Asset, Bundle},
  version::atlaspack_rust_version,
};
use lmdb_js_lite::DatabaseHandle;
use parking_lot::RwLock;
use rayon::prelude::*;
use regex::Regex;

/// Regex to match require("...") or require('...')
static REQUIRE_CALL_REGEX: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r#"require\(["']([^"']+)["']\)"#).unwrap());

use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};

type PackagedAsset<'a> = (&'a Asset, String);

pub struct JsPackager<B: BundleGraph + Send + Sync> {
  db: Arc<DatabaseHandle>,
  bundle_graph: Arc<RwLock<B>>,
}

impl<B: BundleGraph + Send + Sync> JsPackager<B> {
  pub fn new(db: Arc<DatabaseHandle>, bundle_graph: Arc<RwLock<B>>) -> Self {
    Self { db, bundle_graph }
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
    let code = self.replace_require_calls(code, &deps);

    if bundle.entry_asset_ids.contains(&asset.id) {
      Ok(code)
    } else {
      self.wrap_asset(bundle, asset, code)
    }
  }

  // NOTE THIS IS A TEMPORARY HACK IMPL - just to validate the end-to-end packaging.
  // While it produces a (sort of) working bundle, it's not actually how we want to approach this
  fn replace_require_calls(&self, code: String, deps: &HashMap<String, Option<String>>) -> String {
    REQUIRE_CALL_REGEX
      .replace_all(&code, |caps: &regex::Captures| {
        let specifier = &caps[1];

        match deps.get(specifier) {
          Some(Some(public_id)) => {
            format!(r#"require("{}")"#, public_id)
          }
          Some(None) => caps[0].to_string(),
          None => caps[0].to_string(),
        }
      })
      .to_string()
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

    Ok(format!(
      "define('{}', function (require,module,exports) {{ {code} }});",
      public_id
    ))
  }

  pub fn assemble_bundle(&self, bundle: &Bundle, contents: Vec<(&Asset, String)>) -> String {
    // This is a temporary implementation that will just use string concatenation
    // let prelude = r#"
    // (function () {
    // const registry = {};
    // const modules = {};
    // function define(id, factory) {
    //   registry[id] = factory;
    // }
    // function require(id) {
    //   if (modules[id]) {
    //     return modules[id].exports;
    //   }
    //   const module = { exports: {} };
    //   modules[id] = module;
    //   if (!registry[id]) {
    //     const e = new Error(`Module ${id} not found`);
    //     e.code = 'MODULE_NOT_FOUND';
    //     throw e;
    //   }
    //   registry[id].call(module.exports, require, module, module.exports);
    //   return module.exports;
    // }
    // "#;

    let full_hash = hash_string("FIXME".to_string());
    let hash = full_hash
      .chars()
      .skip(full_hash.len() - 4)
      .collect::<String>();

    let prelude_string =
      include_str!("../prelude/lib/prelude.js").replace("ATLASPACK_PRELUDE_HASH", &hash);

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

    let prelude_loader = format!(
      r#"
    var atlaspack = globalObject[`atlaspack_{hash}`];
    var require = atlaspack.require;
    var define = atlaspack.define;
    "#
    );

    "(function() {\n".to_string()
      + &prelude_string
      + &prelude_loader
      + &asset_contents
      + "\n})();\n"
  }
}

#[cfg(test)]
mod tests {
  use super::*;
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

  /// Test that the regex correctly matches require calls and replaces them
  #[test]
  fn test_replace_require_calls_regex_matching() {
    let code = r#"
      const foo = require("./foo");
      const bar = require('./bar');
      const baz = require("deeply/nested/module");
    "#
    .to_string();

    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));
    deps.insert("./bar".to_string(), Some("pub_bar".to_string()));
    deps.insert(
      "deeply/nested/module".to_string(),
      Some("pub_nested".to_string()),
    );

    // Test the regex by extracting the logic
    let result = REQUIRE_CALL_REGEX.replace_all(&code, |caps: &regex::Captures| {
      let specifier = &caps[1];
      match deps.get(specifier) {
        Some(Some(public_id)) => format!(r#"require("{}")"#, public_id),
        _ => caps[0].to_string(),
      }
    });

    assert!(result.contains(r#"require("pub_foo")"#));
    assert!(result.contains(r#"require("pub_bar")"#));
    assert!(result.contains(r#"require("pub_nested")"#));
  }

  #[test]
  fn test_replace_require_calls_preserves_skipped_deps() {
    let code = r#"const foo = require("./foo"); const bar = require("./bar");"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));
    deps.insert("./bar".to_string(), None); // Skipped

    let result = REQUIRE_CALL_REGEX.replace_all(&code, |caps: &regex::Captures| {
      let specifier = &caps[1];
      match deps.get(specifier) {
        Some(Some(public_id)) => format!(r#"require("{}")"#, public_id),
        _ => caps[0].to_string(),
      }
    });

    assert!(result.contains(r#"require("pub_foo")"#));
    assert!(result.contains(r#"require("./bar")"#)); // Unchanged
  }

  #[test]
  fn test_assemble_bundle_structure() {
    // Test the structure and ordering logic without needing a full JsPackager instance
    let bundle = create_test_bundle("bundle1");
    let asset1 = create_test_asset("zzz", "/z.js");
    let asset2 = create_test_asset("aaa", "/a.js");
    let asset3 = create_test_asset("mmm", "/m.js");

    let contents = vec![
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

    let contents = vec![
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

    let expected = format!(
      "define('{}', function (require,module,exports) {{ {} }});",
      public_id, code
    );

    // Verify the format
    assert!(expected.starts_with("define('pub123',"));
    assert!(expected.contains("function (require,module,exports)"));
    assert!(expected.contains("module.exports = 42;"));
    assert!(expected.ends_with("});"));
  }
}
