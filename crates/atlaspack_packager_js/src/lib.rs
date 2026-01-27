use std::sync::Arc;

use atlaspack_core::{bundle_graph::bundle_graph::BundleGraph, types::Asset};
use lmdb_js_lite::DatabaseHandle;
use rayon::prelude::*;

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
  ) -> anyhow::Result<String> {
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
    Ok(contents.join("\n"))
  }
}
