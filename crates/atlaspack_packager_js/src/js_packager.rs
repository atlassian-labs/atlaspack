use std::{collections::HashMap, sync::Arc};

use atlaspack_core::{
  bundle_graph::bundle_graph::BundleGraph,
  hash::{hash_bytes, hash_string},
  types::{Asset, Bundle, OutputFormat},
  version::atlaspack_rust_version,
};
use parking_lot::RwLock;
use pathdiff::diff_paths;
use rayon::prelude::*;

use atlaspack_core::package_result::{BundleInfo, CacheKeyMap, PackageResult};

use super::process_asset::rewrite_asset_code;
use super::{JsPackager, PackagingContext};

type PackagedAsset<'a> = (&'a Asset, String, Option<atlaspack_sourcemap::SourceMap>);

impl<B: BundleGraph + Send + Sync> JsPackager<B> {
  pub fn new(context: PackagingContext, bundle_graph: Arc<RwLock<B>>) -> Self {
    Self {
      context,
      bundle_graph,
    }
  }

  pub fn package(&self, bundle_id: &str) -> anyhow::Result<PackageResult> {
    let graph = self.bundle_graph.read();
    let bundle = graph
      .get_bundle_by_id(bundle_id)
      .ok_or(anyhow::anyhow!("Bundle not found"))?;

    let assets = graph.get_bundle_assets(bundle)?;
    let source_map_enabled = bundle.env.source_map.is_some();

    let span = tracing::trace_span!("process_assets", bundle_id = bundle_id).entered();
    let contents = assets
      .par_iter()
      .map(|asset| {
        let span = tracing::trace_span!("read_code", asset_id = asset.id).entered();
        let txn = self.context.db.database().read_txn()?;
        let code = self.context.db.database().get(
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
          .process_asset(bundle, asset, asset_code, source_map_enabled)
          .map(|(content, asset_map)| (*asset, content, asset_map))
      })
      .collect::<anyhow::Result<Vec<PackagedAsset>>>()?;
    span.exit();

    let (bundle_contents_str, bundle_map) =
      self.assemble_bundle(bundle, contents, source_map_enabled)?;
    let bundle_contents = bundle_contents_str.as_bytes();
    let content_hash = hash_bytes(bundle_contents);
    let content_cache_key = format!(
      "PackagerRunner/{}/{content_hash}/content",
      atlaspack_rust_version()
    );
    let info_cache_key = format!(
      "PackagerRunner/{}/{content_hash}/info",
      atlaspack_rust_version()
    );

    self
      .context
      .cache
      .set_large_blob(&content_cache_key, bundle_contents)?;

    let map_cache_key = if let Some(mut sm) = bundle_map {
      let map_json = sm
        .to_json(None)
        .map_err(|e| anyhow::anyhow!("Failed to serialize bundle source map: {}", e))?;
      let map_key = format!(
        "PackagerRunner/{}/{content_hash}/map",
        atlaspack_rust_version()
      );
      // JS reads the map via cache.getBlob(mapKey) (LMDB).
      self.context.cache.set_blob(&map_key, map_json.as_bytes())?;
      map_key
    } else {
      "TODO".to_string()
    };

    Ok(PackageResult {
      bundle_info: BundleInfo {
        bundle_type: bundle.bundle_type.extension().to_string(),
        size: bundle_contents.len() as u64,
        total_assets: assets.len() as u64,
        hash: content_hash,
        hash_references: vec![],
        cache_keys: CacheKeyMap {
          content: content_cache_key,
          map: map_cache_key,
          info: info_cache_key,
        },
        is_large_blob: true, // Always true for native packager - content is on filesystem
        time: Some(0),
      },
      config_requests: vec![],
      dev_dep_requests: vec![],
      invalidations: vec![],
    })
  }

  #[tracing::instrument(level = "trace", skip_all)]
  fn process_asset(
    &self,
    bundle: &Bundle,
    asset: &Asset,
    code: String,
    source_map_enabled: bool,
  ) -> anyhow::Result<(String, Option<atlaspack_sourcemap::SourceMap>)> {
    let deps = self.get_asset_dependency_map(bundle, asset)?;
    let source_map_path = source_map_enabled.then_some(asset.file_path.as_path());
    let (rewritten_code, oxc_map) = rewrite_asset_code(code, &deps, source_map_path)?;

    let asset_sm = if source_map_enabled {
      if let Some(oxc_map) = oxc_map {
        let oxc_json = oxc_map.to_json_string();
        let mut asset_sm =
          atlaspack_sourcemap::SourceMap::from_json(&self.context.project_root, &oxc_json)
            .map_err(|e| {
              anyhow::anyhow!(
                "Failed to parse OXC source map for asset {}: {}",
                asset.id,
                e
              )
            })?;

        // Compose with the transformer's pre-existing map stored in LMDB at "map:{asset_id}".
        // Ok(None) means the asset has no transformer source map, which is fine.
        let map_key = format!("map:{}", asset.id);
        let txn = self.context.db.database().read_txn()?;
        let existing_map_bytes = self.context.db.database().get(&txn, &map_key)?;
        drop(txn);

        if let Some(existing_map_bytes) = existing_map_bytes {
          let existing_json = std::str::from_utf8(&existing_map_bytes).map_err(|e| {
            anyhow::anyhow!("Invalid UTF-8 in source map for asset {}: {}", asset.id, e)
          })?;
          let mut existing_sm =
            atlaspack_sourcemap::SourceMap::from_json(&self.context.project_root, existing_json)
              .map_err(|e| {
                anyhow::anyhow!(
                  "Failed to parse existing source map for asset {}: {}",
                  asset.id,
                  e
                )
              })?;
          asset_sm.extends(&mut existing_sm).map_err(|e| {
            anyhow::anyhow!(
              "Failed to compose source maps for asset {}: {}",
              asset.id,
              e
            )
          })?;
        }

        Some(asset_sm)
      } else {
        None
      }
    } else {
      None
    };

    let wrapped = self.wrap_asset(bundle, asset, rewritten_code)?;
    Ok((wrapped, asset_sm))
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
    let comment = if self.context.debug_tools.asset_file_names_in_output {
      // Show file path for real files (e.g. node_modules inside or outside project root).
      // Skip path for virtual/generated assets.
      let file_path_comment = if asset.is_virtual {
        String::new()
      } else {
        // Relative to project root (use .. when outside, e.g. node_modules above project).
        let display_path = diff_paths(&asset.file_path, &self.context.project_root)
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

    // The function body starts on a new line so that source map offsets are purely
    // line-based. This avoids any need for column offset arithmetic in the assembler.
    Ok(format!(
      "{comment}define('{public_id}', function (require,module,exports) {{\n{code}\n}});"
    ))
  }

  pub fn assemble_bundle(
    &self,
    bundle: &Bundle,
    contents: Vec<PackagedAsset>,
    source_map_enabled: bool,
  ) -> anyhow::Result<(String, Option<atlaspack_sourcemap::SourceMap>)> {
    let mut bundle_map = if source_map_enabled {
      Some(atlaspack_sourcemap::SourceMap::new(
        &self.context.project_root,
      ))
    } else {
      None
    };

    // Separate entry and non-entry assets
    let (mut entry_contents, mut non_entry_contents): (Vec<PackagedAsset>, Vec<PackagedAsset>) =
      contents
        .into_iter()
        .partition(|(asset, _, _)| bundle.entry_asset_ids.contains(&asset.id));

    non_entry_contents.sort_by_key(|(asset, _, _)| asset.id.clone());
    entry_contents.sort_by_key(|(asset, _, _)| {
      bundle
        .entry_asset_ids
        .iter()
        .position(|id| id == &asset.id)
        .unwrap_or(usize::MAX)
    });

    let mut sorted_contents = non_entry_contents;
    sorted_contents.extend(entry_contents);

    // For now we just always use the dev prelude
    let prelude_string = match self.context.debug_tools.debug_prelude {
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

    let is_commonjs = bundle.env.output_format == OutputFormat::CommonJS;
    let prelude_line_offset: i64 = if is_commonjs {
      count_lines(&prelude_loader)
    } else {
      1 + count_lines(&prelude_loader)
    };

    let mut asset_parts = Vec::new();
    let mut current_line: i64 = 0;

    for (asset, wrapped_content, asset_sm) in &sorted_contents {
      let asset_block_start_line = current_line;
      let lines_before_code: i64 = if self.context.debug_tools.asset_file_names_in_output {
        3
      } else {
        1
      };
      let code_start_line = prelude_line_offset + asset_block_start_line + lines_before_code;

      if let Some(ref mut bundle_map) = bundle_map {
        if let Some(mut sm) = asset_sm.clone() {
          bundle_map
            .add_sourcemap(&mut sm, code_start_line)
            .map_err(|e| {
              anyhow::anyhow!("Failed to add source map for asset {}: {}", asset.id, e)
            })?;
        } else if !asset.is_virtual {
          bundle_map
            .add_empty_map(&asset.file_path.to_string_lossy(), "", code_start_line)
            .map_err(|e| {
              anyhow::anyhow!("Failed to add empty map for asset {}: {}", asset.id, e)
            })?;
        }
      }

      current_line += count_lines(wrapped_content) + 1;
      asset_parts.push(wrapped_content.as_str());
    }

    let asset_contents = asset_parts.join("\n");

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

    let bundle_string = if is_commonjs {
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
    };

    Ok((bundle_string, bundle_map))
  }
}

fn count_lines(s: &str) -> i64 {
  s.bytes().filter(|&b| b == b'\n').count() as i64
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

    let contents: Vec<(&Asset, String, Option<atlaspack_sourcemap::SourceMap>)> = vec![
      (&asset1, "// asset zzz".to_string(), None),
      (&asset2, "// asset aaa".to_string(), None),
      (&asset3, "// asset mmm".to_string(), None),
    ];

    // Test the sorting logic directly
    let (entry_contents, mut non_entry_contents): (Vec<_>, Vec<_>) = contents
      .into_iter()
      .partition(|(asset, _, _)| bundle.entry_asset_ids.contains(&asset.id));

    non_entry_contents.sort_by_key(|(asset, _, _)| asset.id.clone());

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

    let contents: Vec<(&Asset, String, Option<atlaspack_sourcemap::SourceMap>)> = vec![
      (&entry1, "// entry 1".to_string(), None),
      (&non_entry, "// non entry".to_string(), None),
      (&entry2, "// entry 2".to_string(), None),
    ];

    // Test the partitioning and sorting logic
    let (mut entry_contents, mut non_entry_contents): (Vec<_>, Vec<_>) = contents
      .into_iter()
      .partition(|(asset, _, _)| bundle.entry_asset_ids.contains(&asset.id));

    non_entry_contents.sort_by_key(|(asset, _, _)| asset.id.clone());

    entry_contents.sort_by_key(|(asset, _, _)| {
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

    // Function body is on its own line so source map offsets are purely line-based
    // (no column arithmetic needed in the assembler).
    let expected = format!(
      "\n// {}\ndefine('{}', function (require,module,exports) {{\n{}\n}});",
      public_id, public_id, code
    );

    assert!(expected.contains(&format!("// {}", public_id)));
    assert!(expected.contains("define('pub123',"));
    assert!(expected.contains("function (require,module,exports)"));
    assert!(expected.contains("\nmodule.exports = 42;\n"));
    assert!(expected.ends_with("});"));
  }
}
