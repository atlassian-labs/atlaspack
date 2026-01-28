use std::sync::Arc;

use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph, hash::hash_bytes, types::Asset,
  version::atlaspack_rust_version,
};
use lmdb_js_lite::DatabaseHandle;
use rayon::prelude::*;

use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};

pub struct JsPackager {
  db: Arc<DatabaseHandle>,
}

impl JsPackager {
  pub fn new(db: Arc<DatabaseHandle>) -> Self {
    Self { db }
  }

  pub fn package<B: BundleGraph>(
    &self,
    bundle_id: &str,
    bundle_graph: &B,
  ) -> anyhow::Result<PackageResult> {
    let bundle = bundle_graph
      .get_bundle_by_id(bundle_id)
      .ok_or(anyhow::anyhow!("Bundle not found"))?;

    let mut assets: Vec<Asset> = Vec::new();
    // Get all the assets in the bundle
    bundle_graph.traverse_bundle_assets(bundle, &mut |asset: &Asset| {
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
        String::from_utf8_lossy(&code.unwrap()).to_string()
      })
      .collect::<Vec<String>>();

    let bundle_contents = contents.join("\n");
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
        size: contents.into_iter().map(|c| c.len() as u64).sum::<u64>(),
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
}
