use std::{collections::HashMap, io::Write};

use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, AssetNode},
  bundle_graph::BundleGraphBundle,
};
use petgraph::{graph::NodeIndex, visit::EdgeRef};

#[derive(Default, Hash)]
pub struct PackageRequest {}

#[derive(PartialEq, Clone)]
pub struct PackageRequestOutput {}

pub struct PackageBundleParams {
  pub bundle: BundleGraphBundle,
}

pub trait AssetDataProvider {
  fn get_asset_code(&self, asset_id: &str) -> anyhow::Result<Vec<u8>>;
  fn get_original_asset_code(&self, asset_id: &str) -> anyhow::Result<Vec<u8>>;

  fn get_imported_modules(&self, asset_id: &str) -> anyhow::Result<Vec<ImportedModule>>;
  fn get_exported_symbols(&self, asset_id: &str) -> anyhow::Result<Vec<String>>;
}

pub struct InMemoryAssetDataProvider<'a> {
  asset_graph: &'a AssetGraph,
  asset_node_index_by_id: HashMap<String, NodeIndex>,
}

impl<'a> InMemoryAssetDataProvider<'a> {
  pub fn new(asset_graph: &'a AssetGraph) -> Self {
    let mut asset_node_index_by_id = HashMap::new();

    for node_index in asset_graph.graph.node_indices() {
      let asset_node = asset_graph.get_node(&node_index).unwrap();
      if let AssetGraphNode::Asset(asset_node) = asset_node {
        asset_node_index_by_id.insert(asset_node.asset.id.clone(), node_index);
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
      .get(asset_id)
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
      .get(asset_id)
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
        let target_index = targets.next().unwrap().target();
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

pub fn package_bundle(
  PackageBundleParams { bundle }: PackageBundleParams,
  asset_data_provider: impl AssetDataProvider,
  writer: &mut impl Write,
) -> anyhow::Result<()> {
  for asset_id in bundle.assets.iter() {
    let code = String::from_utf8(asset_data_provider.get_asset_code(asset_id)?)?;

    let import_statements = asset_data_provider.get_imported_modules(asset_id)?;
    let export_statements: Vec<String> = vec![]; // asset_data_provider.get_exported_symbols(asset_id)?;

    writer.write_all(
      format!(
        "atlaspack$register('{}', function(atlaspack$require, atlaspack$export) {{\n\n\n",
        asset_id
      )
      .as_bytes(),
    )?;

    for import_statement in import_statements {
      writer.write_all(
        format!(
          "const {{ {} }} = atlaspack$require('{}');\n",
          import_statement.symbols.join(", "),
          import_statement.target_id,
        )
        .as_bytes(),
      )?;
    }

    writer.write_all(code.as_bytes())?;

    for export_statement in export_statements {
      writer.write_all(format!("atlaspack$export('{}', {{}});\n", export_statement).as_bytes())?;
    }

    writer.write_all("\n\n}});\n\n".as_bytes());
  }

  Ok(())
}
