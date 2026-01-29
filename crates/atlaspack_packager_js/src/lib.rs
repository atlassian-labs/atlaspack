use std::sync::{Arc, RwLock};

use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph,
  hash::hash_bytes,
  types::{Asset, Bundle},
  version::atlaspack_rust_version,
};
use lmdb_js_lite::DatabaseHandle;
use rayon::prelude::*;

use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};

use crate::assemble::assemble_bundle;

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
      tracing::debug!("Traversing asset: {}", asset.id);
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
        if bundle.entry_asset_ids.contains(&asset.id) {
          Ok(asset_code)
        } else {
          self.wrap_asset(bundle, asset, asset_code)
        }
      })
      .collect::<anyhow::Result<Vec<String>>>()?;

    let bundle_contents = assemble_bundle(contents);
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

  fn wrap_asset(&self, bundle: &Bundle, asset: &Asset, code: String) -> anyhow::Result<String> {
    let bundle_graph = self.bundle_graph.read().unwrap();
    let public_id = bundle_graph
      .get_public_asset_id(&asset.id)
      .expect("Asset not found in bundle graph")
      .to_string();

    // Get dependencies for asset
    let dependencies = bundle_graph.get_dependencies(asset)?;
    for dependency in dependencies {
      let resolved = bundle_graph.get_resolved_asset(dependency, bundle)?;

      let specifier = match dependency.meta.get("placeholder") {
        Some(placeholder) => placeholder.as_str().unwrap(),
        None => &dependency.specifier,
      };

      let dep_value: Option<&str> = if bundle_graph.is_dependency_skipped(dependency) {
        None
      } else if let Some(resolved) = resolved {
        Some(
          bundle_graph
            .get_public_asset_id(&resolved.id)
            .ok_or(anyhow::anyhow!("Asset not found in bundle graph"))?,
        )
      } else {
        tracing::debug!(
          "Dependency {} did not resolve to an asset in the bundle graph",
          dependency.id
        );
        Some(&dependency.specifier)
      };
      dbg!(&specifier, &dep_value);
    }
    println!("----");
    Ok(format!(
      "define('{}', function (require,module,exports) {{ {code} }});",
      public_id
    ))
  }
}
