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
  bundle_graph::{BundleGraph, BundleGraphBundle, BundleGraphNode},
  plugin::PackageContext,
  types::{AssetId, AtlaspackOptions, Bundle, BundleId, FileType},
};
use petgraph::{graph::NodeIndex, visit::EdgeRef};
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use tracing::{info, warn};

use crate::{
  request_tracker::{
    Request, RequestId, ResultAndInvalidations, RunRequestContext, RunRequestError,
  },
  requests::RequestResult,
};

#[derive(Debug)]
pub struct PackageRequest {
  bundle_graph: BundleGraph,
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
      bundle_graph,
      asset_graph,
    }
  }
}

#[async_trait]
impl Request for PackageRequest {
  async fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let bundles = self
      .bundle_graph
      .graph()
      .node_weights()
      .filter_map(|weight| {
        let BundleGraphNode::Bundle(bundle) = weight else {
          return None;
        };

        Some(bundle)
      })
      .collect::<Vec<_>>();

    let results = bundles
      .into_par_iter()
      .map(|bundle| {
        let options = request_context.options.clone();
        let project_root = request_context.project_root.clone();
        let asset_graph = self.asset_graph.clone();

        run_package_bundle(options, project_root, bundle.clone(), asset_graph)
      })
      .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ResultAndInvalidations {
      result: RequestResult::Package(PackageRequestOutput {
        packaged_bundle: PathBuf::from(""),
      }),
      invalidations: vec![],
    })
  }
}

fn run_package_bundle(
  options: Arc<AtlaspackOptions>,
  project_root: PathBuf,
  bundle: BundleGraphBundle,
  asset_graph: Arc<AssetGraph>,
) -> Result<ResultAndInvalidations, RunRequestError> {
  // We should not keep all data in memory
  let asset_data_provider = InMemoryAssetDataProvider::new(&asset_graph);

  let output_file_path = options
    .default_target_options
    .dist_dir
    .clone()
    .unwrap_or_else(|| project_root.join("dist"))
    .join(bundle.bundle.name.clone().unwrap_or_else(|| {
      format!(
        "{}.{}",
        bundle.bundle.id,
        bundle.bundle.bundle_type.extension()
      )
    }));

  info!("Packaging bundle to {}", output_file_path.display());
  let mut writer = std::fs::File::create(&output_file_path)?;
  package_bundle(
    PackageBundleParams { bundle: &bundle },
    asset_data_provider,
    &mut writer,
  )?;

  Ok(ResultAndInvalidations {
    result: RequestResult::Package(PackageRequestOutput {
      packaged_bundle: output_file_path,
    }),
    invalidations: vec![],
  })
}

#[derive(PartialEq, Clone)]
pub struct PackageRequestOutput {
  packaged_bundle: PathBuf,
}

pub struct PackageBundleParams<'a> {
  pub bundle: &'a BundleGraphBundle,
}

pub trait AssetDataProvider: std::fmt::Debug {
  fn get_asset_code(&self, asset_id: &str) -> anyhow::Result<Vec<u8>>;
  fn get_original_asset_code(&self, asset_id: &str) -> anyhow::Result<Vec<u8>>;
  fn get_imported_modules(&self, asset_id: &str) -> anyhow::Result<Vec<ImportedModule>>;
  fn get_exported_symbols(&self, asset_id: &str) -> anyhow::Result<Vec<String>>;
}

#[derive(Debug)]
pub struct InMemoryAssetDataProvider<'a> {
  asset_graph: &'a AssetGraph,
  asset_node_index_by_id: HashMap<u64, NodeIndex>,
}

fn get_asset_id_hash(asset_id: &str) -> u64 {
  let mut hasher = atlaspack_core::hash::IdentifierHasher::default();
  asset_id.hash(&mut hasher);
  hasher.finish()
}

impl<'a> InMemoryAssetDataProvider<'a> {
  pub fn new(asset_graph: &'a AssetGraph) -> Self {
    let mut asset_node_index_by_id = HashMap::new();

    for node_index in asset_graph.graph.node_indices() {
      let asset_node = asset_graph.get_node(&node_index).unwrap();
      if let AssetGraphNode::Asset(asset_node) = asset_node {
        let id_hash = get_asset_id_hash(&asset_node.asset.id);
        asset_node_index_by_id.insert(id_hash, node_index);
      }
    }

    Self {
      asset_graph,
      asset_node_index_by_id,
    }
  }
}

impl InMemoryAssetDataProvider<'_> {
  fn get_asset_by_id(&self, asset_id: &str) -> anyhow::Result<&AssetNode> {
    let asset_node_index = self
      .asset_node_index_by_id
      .get(&get_asset_id_hash(asset_id))
      .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

    let AssetGraphNode::Asset(asset_node) = self.asset_graph.get_node(asset_node_index).unwrap()
    else {
      anyhow::bail!("Asset not found: {}", asset_id);
    };

    Ok(asset_node)
  }
}

struct ImportedModule {
  target_id: String,
  specifier: String,
  symbols: Vec<String>,
}

impl AssetDataProvider for InMemoryAssetDataProvider<'_> {
  fn get_original_asset_code(&self, asset_id: &str) -> anyhow::Result<Vec<u8>> {
    let asset_node = self.get_asset_by_id(asset_id)?;
    let path = asset_node.asset.file_path.clone();
    let code = std::fs::read(path)?;
    Ok(code)
  }

  fn get_asset_code(&self, asset_id: &str) -> anyhow::Result<Vec<u8>> {
    let asset_node = self.get_asset_by_id(asset_id)?;
    Ok(asset_node.asset.code.bytes().to_vec())
  }

  fn get_imported_modules(&self, asset_id: &str) -> anyhow::Result<Vec<ImportedModule>> {
    let asset_node_index = self
      .asset_node_index_by_id
      .get(&get_asset_id_hash(asset_id))
      .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

    let AssetGraphNode::Asset(asset_node) = self.asset_graph.get_node(asset_node_index).unwrap()
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

  fn get_exported_symbols(&self, asset_id: &str) -> anyhow::Result<Vec<String>> {
    let asset_node = self.get_asset_by_id(asset_id)?;
    let Some(symbols) = &asset_node.asset.symbols else {
      return Ok(vec![]);
    };

    Ok(symbols.iter().map(|s| s.local.clone()).collect())
  }
}

#[tracing::instrument(skip(params, asset_data_provider, writer), fields(bundle_id = %params.bundle.bundle.id))]
pub fn package_bundle(
  params: PackageBundleParams<'_>,
  asset_data_provider: impl AssetDataProvider,
  writer: &mut impl Write,
) -> anyhow::Result<()> {
  match params.bundle.bundle.bundle_type {
    FileType::Js => package_js_bundle(params, asset_data_provider, writer),
    FileType::Html => package_html_bundle(params, asset_data_provider, writer),
    _ => {
      warn!(
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
  PackageBundleParams { bundle }: PackageBundleParams,
  asset_data_provider: impl AssetDataProvider,
  writer: &mut impl Write,
) -> anyhow::Result<()> {
  for asset_id in bundle.assets.iter() {
    let code = asset_data_provider.get_asset_code(asset_id)?;
    writer.write_all(&code)?;
  }
  Ok(())
}

fn package_js_bundle(
  PackageBundleParams { bundle }: PackageBundleParams<'_>,
  asset_data_provider: impl AssetDataProvider,
  writer: &mut impl Write,
) -> anyhow::Result<()> {
  for asset_id in bundle.assets.iter() {
    let code = String::from_utf8(asset_data_provider.get_asset_code(asset_id)?)?;

    let import_statements = asset_data_provider.get_imported_modules(asset_id)?;
    let export_statements: Vec<String> = vec![]; // asset_data_provider.get_exported_symbols(asset_id)?;

    writer.write_all(
      format!(
        "atlaspack$register('{}', function(atlaspack$require, atlaspack$export, exports) {{\n\n\n",
        asset_id
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

    let postlude = format!("\n\n}}, {dependencies_object_string});\n\n");
    writer.write_all(postlude.as_bytes())?;
  }

  Ok(())
}
