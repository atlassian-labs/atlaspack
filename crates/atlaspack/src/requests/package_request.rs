use std::{
  collections::HashMap,
  hash::{Hash, Hasher},
  io::Write,
  path::PathBuf,
  sync::Arc,
};

use async_trait::async_trait;
use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, AssetNode},
  bundle_graph::{BundleGraph, BundleGraphBundle, BundleGraphEdge, BundleGraphNode},
  plugin::PackagerPlugin,
  types::{AssetId, AtlaspackOptions, BundleId, FileType},
};
use petgraph::{
  graph::NodeIndex,
  visit::{EdgeRef, IntoNodeReferences},
};
use rayon::prelude::*;
use tracing::{debug, info_span, warn};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::RequestResult,
};

#[derive(Debug)]
pub struct PackageRequest {
  bundle_graph: Arc<BundleGraph>,
  asset_graph: Arc<AssetGraph>,
}

impl Hash for PackageRequest {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.asset_graph.hash(state);
  }
}

impl PackageRequest {
  pub fn new(bundle_graph: BundleGraph, asset_graph: Arc<AssetGraph>) -> Self {
    Self {
      bundle_graph: Arc::new(bundle_graph),
      asset_graph,
    }
  }
}

#[async_trait]
impl Request for PackageRequest {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let bundles = self
      .bundle_graph
      .graph()
      .node_references()
      .filter_map(|(node_index, weight)| {
        let BundleGraphNode::Bundle(bundle) = weight else {
          return None;
        };

        Some((node_index, bundle.clone()))
      })
      .collect::<Vec<_>>();

    let package_bundles_span = info_span!("Packaging bundles", indicatif.pb_show = true);
    package_bundles_span.pb_set_message("Packaging bundles...");
    package_bundles_span.pb_set_length(bundles.len() as u64);
    // Not sure if this is correct
    let _ = package_bundles_span.enter();

    let asset_data_provider = Arc::new(InMemoryAssetDataProvider::new(self.asset_graph.clone()));

    let results = bundles
      .into_par_iter()
      .map({
        let bundle_graph = self.bundle_graph.clone();
        let asset_data_provider = asset_data_provider.clone();

        move |(node_index, bundle)| {
          let options = request_context.options.clone();
          let project_root = request_context.project_root.clone();

          let output_file_path = get_bundle_file_path(&options, &project_root, &bundle);
          let packager = request_context.plugins().packager(&output_file_path).ok();

          let package_result = run_package_bundle(
            output_file_path.clone(),
            options,
            project_root,
            node_index,
            &bundle,
            &*asset_data_provider,
            &*bundle_graph,
            packager,
          )?;

          package_bundles_span.pb_inc(1);

          Ok((package_result, (bundle.bundle.id.clone(), output_file_path)))
        }
      })
      .collect::<anyhow::Result<Vec<_>>>()?;

    let bundle_paths = results
      .into_iter()
      .map(|(_, (id, path))| (id, path))
      .collect();

    Ok(ResultAndInvalidations {
      result: RequestResult::Package(PackageRequestOutput { bundle_paths }),
      invalidations: vec![],
    })
  }
}

fn get_bundle_file_path(
  options: &AtlaspackOptions,
  project_root: &PathBuf,
  bundle: &BundleGraphBundle,
) -> PathBuf {
  let dist_dir = options
    .default_target_options
    .dist_dir
    .clone()
    .unwrap_or_else(|| project_root.join("dist"));

  let file_name = bundle.bundle.name.clone().unwrap_or_else(|| {
    format!(
      "{}.{}",
      bundle.bundle.id,
      bundle.bundle.bundle_type.extension()
    )
  });

  dist_dir.join(file_name)
}

fn run_package_bundle(
  output_file_path: PathBuf,
  _options: Arc<AtlaspackOptions>,
  _project_root: PathBuf,
  bundle_node_index: NodeIndex,
  bundle: &BundleGraphBundle,
  asset_data_provider: &impl AssetDataProvider,
  bundle_graph: &BundleGraph,
  packager: Option<Box<dyn PackagerPlugin>>,
) -> Result<ResultAndInvalidations, RunRequestError> {
  debug!("Packaging bundle to {}", output_file_path.display());
  std::fs::create_dir_all(&output_file_path.parent().unwrap())?;
  let mut writer = std::fs::File::create(&output_file_path)?;
  package_bundle(
    PackageBundleParams {
      bundle: &bundle,
      bundle_node_index,
    },
    asset_data_provider,
    &mut writer,
    bundle_graph,
    packager,
  )?;

  Ok(ResultAndInvalidations {
    result: RequestResult::Package(PackageRequestOutput {
      bundle_paths: HashMap::from([(bundle.bundle.id.clone(), output_file_path)]),
    }),
    invalidations: vec![],
  })
}

#[derive(PartialEq, Clone)]
pub struct PackageRequestOutput {
  pub bundle_paths: HashMap<BundleId, PathBuf>,
}

pub struct PackageBundleParams<'a> {
  pub bundle: &'a BundleGraphBundle,
  pub bundle_node_index: NodeIndex,
}

pub trait AssetDataProvider: std::fmt::Debug {
  fn get_asset_code(&self, asset_id: AssetId) -> anyhow::Result<Vec<u8>>;
  fn get_original_asset_code(&self, asset_id: AssetId) -> anyhow::Result<Vec<u8>>;
  fn get_imported_modules(&self, asset_id: AssetId) -> anyhow::Result<Vec<ImportedModule>>;
  fn get_exported_symbols(&self, asset_id: AssetId) -> anyhow::Result<Vec<String>>;
}

#[derive(Debug)]
pub struct InMemoryAssetDataProvider {
  asset_graph: Arc<AssetGraph>,
  asset_node_index_by_id: HashMap<AssetId, NodeIndex>,
}

impl InMemoryAssetDataProvider {
  pub fn new(asset_graph: Arc<AssetGraph>) -> Self {
    let mut asset_node_index_by_id = HashMap::new();

    for node_index in asset_graph.graph.node_indices() {
      let asset_node = asset_graph.get_node(&node_index).unwrap();
      if let AssetGraphNode::Asset(asset_node) = asset_node {
        let id_hash = asset_node.asset.id();
        asset_node_index_by_id.insert(*id_hash, node_index);
      }
    }

    Self {
      asset_graph,
      asset_node_index_by_id,
    }
  }
}

impl InMemoryAssetDataProvider {
  fn get_asset_by_id(&self, asset_id: AssetId) -> anyhow::Result<&AssetNode> {
    let asset_node_index = self
      .asset_node_index_by_id
      .get(&asset_id)
      .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

    let AssetGraphNode::Asset(asset_node) = self.asset_graph.get_node(asset_node_index).unwrap()
    else {
      anyhow::bail!("Asset not found: {}", asset_id);
    };

    Ok(asset_node)
  }
}

struct ImportedModule {
  target_id: AssetId,
  specifier: String,
  symbols: Vec<String>,
}

impl AssetDataProvider for InMemoryAssetDataProvider {
  fn get_original_asset_code(&self, asset_id: AssetId) -> anyhow::Result<Vec<u8>> {
    let asset_node = self.get_asset_by_id(asset_id)?;
    let path = asset_node.asset.file_path.clone();
    let code = std::fs::read(path)?;
    Ok(code)
  }

  fn get_asset_code(&self, asset_id: AssetId) -> anyhow::Result<Vec<u8>> {
    let asset_node = self.get_asset_by_id(asset_id)?;
    Ok(asset_node.asset.code.bytes().to_vec())
  }

  fn get_imported_modules(&self, asset_id: AssetId) -> anyhow::Result<Vec<ImportedModule>> {
    let asset_node_index = self
      .asset_node_index_by_id
      .get(&asset_id)
      .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

    let AssetGraphNode::Asset(_asset_node) = self.asset_graph.get_node(asset_node_index).unwrap()
    else {
      anyhow::bail!("Asset not found: {}", asset_id);
    };

    let edges = self
      .asset_graph
      .graph
      .edges_directed(*asset_node_index, petgraph::Direction::Outgoing);

    let mut result = vec![];

    for edge in edges {
      let target = edge.target();
      let target_node = self.asset_graph.graph.node_weight(target).unwrap();

      if let AssetGraphNode::Dependency(dependency_node) = target_node {
        let mut targets = self
          .asset_graph
          .graph
          .edges_directed(target, petgraph::Direction::Outgoing);
        let Some(target_index) = targets.next() else {
          warn!("Disconnected dependency: {:?}", dependency_node.dependency);
          continue;
        };
        let target_index = target_index.target();
        let AssetGraphNode::Asset(target_node) =
          self.asset_graph.graph.node_weight(target_index).unwrap()
        else {
          panic!("Target node is not an asset");
        };

        let dependency_asset_id = target_node.asset.id.clone();

        result.push(ImportedModule {
          target_id: dependency_asset_id,
          symbols: dependency_node
            .dependency
            .symbols
            .as_ref()
            .map(|symbols| symbols.iter().map(|s| s.local.clone()).collect())
            .unwrap_or_default(),
          specifier: dependency_node.dependency.specifier.clone(),
        });
      }
    }

    Ok(result)
  }

  fn get_exported_symbols(&self, asset_id: AssetId) -> anyhow::Result<Vec<String>> {
    let asset_node = self.get_asset_by_id(asset_id)?;
    let Some(symbols) = &asset_node.asset.symbols else {
      return Ok(vec![]);
    };

    Ok(symbols.iter().map(|s| s.local.clone()).collect())
  }
}

#[tracing::instrument(skip(params, asset_data_provider, writer), fields(bundle_id = %params.bundle.bundle.id), level = "debug")]
pub fn package_bundle(
  params: PackageBundleParams<'_>,
  asset_data_provider: &impl AssetDataProvider,
  writer: &mut impl Write,
  bundle_graph: &BundleGraph,
  packager: Option<Box<dyn PackagerPlugin>>,
) -> anyhow::Result<()> {
  if let Some(_packager) = packager {
    debug!("Have packager for {:?}", params.bundle.bundle.bundle_type);
    // packager.package(PackageContext {
    //   bundle: &params.bundle.bundle,
    //   bundle_graph: &params.bundle.assets,
    //   contents: writer,
    //   map: None,
    // })?;
  }

  match params.bundle.bundle.bundle_type {
    FileType::Js => package_js_bundle(params, asset_data_provider, writer, bundle_graph),
    FileType::Html => package_html_bundle(params, asset_data_provider, writer, bundle_graph),
    _ => {
      debug!(
        "Unsupported bundle type: {:?}",
        params.bundle.bundle.bundle_type
      );
      Ok(())
    } // _ => Err(anyhow::anyhow!(
      //   "Unsupported bundle type: {:?}",
      //   params.bundle.bundle.bundle_type
      // )),
  }
}

fn package_html_bundle(
  PackageBundleParams {
    bundle,
    bundle_node_index,
  }: PackageBundleParams,
  asset_data_provider: &impl AssetDataProvider,
  writer: &mut impl Write,
  bundle_graph: &BundleGraph,
) -> anyhow::Result<()> {
  assert!(bundle.assets.node_count() == 1);
  let bundle_asset_ref = bundle.assets.node_weights().next().unwrap();
  let bundle_asset = asset_data_provider.get_asset_code(bundle_asset_ref.id())?;

  let prelude = include_str!("./packager_runtime/prelude.js");
  writer.write_all(format!("<script>{}</script>", prelude).as_bytes())?;

  let referenced_bundles = bundle_graph
    .graph()
    .edges_directed(bundle_node_index, petgraph::Direction::Outgoing)
    .map(|e| (e.target(), e.weight()))
    .collect::<Vec<_>>();

  // println!(
  //   "referenced_bundles: {} count={}",
  //   bundle,
  //   referenced_bundles.len()
  // );

  let mut bundle_asset_string = String::from_utf8(bundle_asset)?;

  for (referenced_bundle_node_index, edge_weight) in referenced_bundles {
    let bundle_node = bundle_graph
      .graph()
      .node_weight(referenced_bundle_node_index)
      .unwrap();

    // println!("  referenced_bundle: {}", bundle_node);
    let BundleGraphNode::Bundle(bundle) = bundle_node else {
      panic!("Referenced bundle is not a bundle: {:?}", bundle_node);
    };
    let dependency_id = match edge_weight {
      BundleGraphEdge::BundleSyncLoads(dependency_node) => dependency_node.id(),
      BundleGraphEdge::BundleAsyncLoads(dependency_node) => dependency_node.id(),
      _ => unreachable!(),
    };

    // find the reference ID
    let regex = regex::Regex::new(&format!("{}", dependency_id)).unwrap();
    let matches = regex.find(&bundle_asset_string);
    if let Some(m) = matches {
      // println!("  found match: {}", m.as_str());
      bundle_asset_string = bundle_asset_string.replace(
        m.as_str(),
        // TODO: Public path
        &format!("{}", bundle.bundle.name.as_ref().unwrap().as_str()),
      );
    }
  }

  writer.write_all(bundle_asset_string.as_bytes())?;

  Ok(())
}

fn package_js_bundle(
  PackageBundleParams {
    bundle,
    bundle_node_index,
  }: PackageBundleParams<'_>,
  asset_data_provider: &impl AssetDataProvider,
  writer: &mut impl Write,
  bundle_graph: &BundleGraph,
) -> anyhow::Result<()> {
  let mut sorted_indexes =
    petgraph::algo::toposort(&bundle.assets, None).expect("Cycle in bundle graph");
  sorted_indexes.reverse();

  writer.write_all("const ms = (window.atlaspack$ms = window.atlaspack$ms || []);\n".as_bytes())?;

  for node_id in sorted_indexes {
    let asset_ref = bundle.assets.node_weight(node_id).unwrap();
    let code = String::from_utf8(asset_data_provider.get_asset_code(asset_ref.id())?)?;

    let import_statements = asset_data_provider.get_imported_modules(asset_ref.id())?;
    let _export_statements: Vec<String> = vec![]; // asset_data_provider.get_exported_symbols(asset_id)?;

    writer.write_all(
      format!(
        "ms.push(['{}', (exports, require, atlaspack$require, atlaspack$export) => {{\n\n\n",
        asset_ref.id()
      )
      .as_bytes(),
    )?;

    // If scope hoisting:
    // for import_statement in import_statements {
    //   writer.write_all(
    //     format!(
    //       "const {{ {} }} = atlaspack$require('{}');\n",
    //       import_statement.symbols.join(", "),
    //       import_statement.target_id,
    //     )
    //     .as_bytes(),
    //   )?;
    // }

    writer.write_all(code.as_bytes())?;

    // If scope hoisting:
    // for export_statement in export_statements {
    //   writer.write_all(format!("atlaspack$export('{}', {{}});\n", export_statement).as_bytes())?;
    // }

    let mut dependencies_object = HashMap::new();
    for import_statement in import_statements {
      dependencies_object.insert(import_statement.specifier, import_statement.target_id);
    }
    let dependencies_object_string = serde_json::to_string(&dependencies_object).unwrap();

    let postlude = format!("\n\n}}, {dependencies_object_string},\n\n ]);\n\n");
    writer.write_all(postlude.as_bytes())?;
  }

  let referenced_bundles = bundle_graph
    .graph()
    .edges_directed(bundle_node_index, petgraph::Direction::Outgoing)
    .map(|e| (e.target(), e.weight()))
    .collect::<Vec<_>>();

  // println!(
  //   "referenced_bundles: {} count={}",
  //   bundle,
  //   referenced_bundles.len()
  // );

  for (referenced_bundle_node_index, edge_weight) in referenced_bundles {
    let bundle_node = bundle_graph
      .graph()
      .node_weight(referenced_bundle_node_index)
      .unwrap();

    // println!("  referenced_bundle: {}", bundle_node);
    let BundleGraphNode::Bundle(bundle) = bundle_node else {
      panic!("Referenced bundle is not a bundle: {:?}", bundle_node);
    };
    let dependency = match edge_weight {
      BundleGraphEdge::BundleAsyncLoads(dependency_node) => dependency_node,
      _ => continue,
    };

    let Some(placeholder) = dependency.placeholder() else {
      // TODO: This is a problem, but in general it's happening with SVGs, images and so on, because that isn't handled yet.
      tracing::debug!("Dependency has no placeholder but needed to be async imported from bundle={}: dependency={}", bundle, dependency);
      continue;
    };

    // writer.write_all(
    //   format!(
    //     "ms.push(['{}', (exports, require, atlaspack$require, atlaspack$export) => {{\n\n\n",
    //     placeholder
    //   )
    //   .as_bytes(),
    // )?;
    // writer.write_all(
    //   format!(
    //     "exports.default = import(new URL('{}', import.meta.url)).then(() => atlaspack$require('{}'));\n\n",
    //     bundle.bundle.name.as_ref().unwrap(),
    //     dependency.id(),
    //   )
    //   .as_bytes(),
    // )?;
    // writer.write_all("}]);\n\n".as_bytes())?;
  }

  writer.write_all("atlaspack$bootstrap();".as_bytes())?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_package_html_bundle() {
    let bundle_graph = BundleGraph::new();
  }
}
