use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph,
  hash::hash_bytes,
  types::{Asset, Bundle},
  version::atlaspack_rust_version,
};
use lmdb_js_lite::DatabaseHandle;
use rayon::prelude::*;
use regex::Regex;

use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};

mod assemble;

pub struct JsPackager<B: BundleGraph + Send + Sync> {
  db: Arc<DatabaseHandle>,
  bundle_graph: Arc<RwLock<B>>,
}

impl<B: BundleGraph + Send + Sync> JsPackager<B> {
  pub fn new(db: Arc<DatabaseHandle>, bundle_graph: Arc<RwLock<B>>) -> Self {
    Self { db, bundle_graph }
  }

  /// Acquires a read lock and returns a guard. Use for operations that need the graph for
  /// multiple calls (e.g. get_bundle_by_id then traverse_bundle_assets). For single lookups
  /// from other threads (e.g. in par_iter), use `self.bundle_graph.read().unwrap()` directly.
  fn bundle_graph(&self) -> std::sync::RwLockReadGuard<'_, B> {
    self.bundle_graph.read().unwrap()
  }

  pub fn package(&self, bundle_id: &str) -> anyhow::Result<PackageResult> {
    let graph = self.bundle_graph();
    let bundle = graph
      .get_bundle_by_id(bundle_id)
      .ok_or(anyhow::anyhow!("Bundle not found"))?;

    let mut assets: Vec<Asset> = Vec::new();
    // Get all the assets in the bundle
    graph.traverse_bundle_assets(bundle, &mut |asset: &Asset| {
      // tracing::debug!("Traversing asset: {} for bundle {}", asset.id, bundle_id);
      assets.push(asset.clone());
    });
    let contents = assets
      .par_iter()
      .map(|asset| {
        let txn = self.db.database().read_txn().unwrap();
        let code = self
          .db
          .database()
          .get(&txn, asset.content_key.as_ref().unwrap())
          .unwrap();
        let asset_code = String::from_utf8_lossy(&code.unwrap()).to_string();
        self
          .process_asset(bundle, asset, asset_code)
          .map(|content| (asset, content))
      })
      .collect::<anyhow::Result<Vec<(&Asset, String)>>>()?;

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

    tracing::debug!("Content cache key: {}", content_cache_key);
    let mut write_txn = self.db.database().write_txn()?;
    self
      .db
      .database()
      .put(&mut write_txn, &content_cache_key, bundle_contents)?;

    // As the "info" object needs to be read from JS, it needs to be serialized by JS - for now
    // we return it to JS and write it to LMDB there..

    write_txn.commit().unwrap();

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

  fn process_asset(&self, bundle: &Bundle, asset: &Asset, code: String) -> anyhow::Result<String> {
    tracing::debug!("Visiting asset: {} for bundle {}", asset.id, bundle.id);

    // Get dependency map for this asset
    let deps = self.get_asset_dependency_map(bundle, asset)?;
    let code = self.replace_require_calls(code, &deps);
    // Entry assets are not wrapped but need require() calls replaced
    if bundle.entry_asset_ids.contains(&asset.id) {
      Ok(code)
    } else {
      self.wrap_asset(bundle, asset, code)
    }
  }

  fn replace_require_calls(&self, code: String, deps: &HashMap<String, Option<String>>) -> String {
    // Regex to match require("...") or require('...')
    let re = Regex::new(r#"require\(["']([^"']+)["']\)"#).unwrap();

    re.replace_all(&code, |caps: &regex::Captures| {
      let specifier = &caps[1];

      match deps.get(specifier) {
        Some(Some(public_id)) => {
          tracing::debug!(
            "Replacing require(\"{}\") with require(\"{}\")",
            specifier,
            public_id
          );
          format!(r#"require("{}")"#, public_id)
        }
        Some(None) => {
          tracing::warn!(
            "Dependency \"{}\" was skipped, leaving unreplaced",
            specifier
          );
          caps[0].to_string()
        }
        None => {
          tracing::warn!(
            "No dependency found for specifier \"{}\", leaving unreplaced",
            specifier
          );
          caps[0].to_string()
        }
      }
    })
    .to_string()
  }

  fn get_asset_dependency_map(
    &self,
    bundle: &Bundle,
    asset: &Asset,
  ) -> anyhow::Result<HashMap<String, Option<String>>> {
    let bundle_graph = self.bundle_graph.read().unwrap();

    // Get dependencies for asset
    let dependencies = bundle_graph.get_dependencies(asset)?;
    let mut deps: HashMap<String, Option<String>> = HashMap::new();

    for dependency in dependencies {
      let resolved = bundle_graph.get_resolved_asset(dependency, bundle)?;

      let specifier = dependency
        .placeholder
        .as_deref()
        .unwrap_or(&dependency.specifier);

      let dep_value: Option<String> = if bundle_graph.is_dependency_skipped(dependency) {
        None
      } else if let Some(resolved) = resolved {
        Some(
          bundle_graph
            .get_public_asset_id(&resolved.id)
            .ok_or(anyhow::anyhow!("Asset not found in bundle graph"))?
            .to_string(),
        )
      } else {
        tracing::debug!(
          "Dependency {} did not resolve to an asset in the bundle graph",
          dependency.id
        );
        Some(dependency.specifier.clone())
      };

      deps.insert(specifier.to_string(), dep_value);
    }

    tracing::debug!("Asset {} dependencies done", asset.id);
    Ok(deps)
  }

  fn wrap_asset(&self, _bundle: &Bundle, asset: &Asset, code: String) -> anyhow::Result<String> {
    let bundle_graph = self.bundle_graph.read().unwrap();
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
    let prelude = r#"
    (function () {
    const registry = {};
    const modules = {};
    function define(id, factory) {
      registry[id] = factory;
    }
    function require(id) {
      if (modules[id]) {
        return modules[id].exports;
      }
      const module = { exports: {} };
      modules[id] = module;
      if (!registry[id]) {
        const e = new Error(`Module ${id} not found`);
        e.code = 'MODULE_NOT_FOUND';
        throw e;
      }
      registry[id].call(module.exports, require, module, module.exports);
      return module.exports;
    }
    "#;

    // Sort the contents - non-entry assets by asset id first, then entry assets in the same order as bundle.entry_asset_ids

    // Separate entry and non-entry assets
    let (mut entry_contents, mut non_entry_contents): (
      Vec<(&Asset, String)>,
      Vec<(&Asset, String)>,
    ) = contents
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
    prelude.to_string() + &asset_contents + "\n})();\n"
  }
}
