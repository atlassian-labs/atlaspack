use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode},
  bundle_graph::{NativeBundleGraph, native_bundle_graph::NativeBundleGraphEdgeType},
  hash::hash_string,
  types::{Bundle, FileType, Target},
};

/// Bundler algorithms take an asset graph and assign assets/dependencies to bundles.
///
/// Implementations are expected to mutate the provided `NativeBundleGraph` to:
/// - create bundle / bundle_group nodes (`bundle_nodes`)
/// - create bundle membership edges (`bundle_edges`)
pub trait Bundler {
  fn bundle(
    &self,
    asset_graph: &AssetGraph,
    bundle_graph: &mut NativeBundleGraph,
  ) -> anyhow::Result<()>;
}

/// Milestone 1: monolithic SSR bundling.
///
/// Assigns all reachable JS assets to a single JS bundle.
#[derive(Debug, Default)]
pub struct MonolithicBundler;

impl Bundler for MonolithicBundler {
  fn bundle(
    &self,
    asset_graph: &AssetGraph,
    bundle_graph: &mut NativeBundleGraph,
  ) -> anyhow::Result<()> {
    // Find the entry dependency + entry asset for the main bundle.
    let mut entry_dep: Option<atlaspack_core::types::Dependency> = None;
    let mut entry_asset_id: Option<String> = None;

    for node in asset_graph.nodes() {
      if let AssetGraphNode::Dependency(dep) = node {
        if dep.is_entry {
          entry_dep = Some((**dep).clone());
          // Resolve to first outgoing asset
          if let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(&dep.id) {
            let neighbors = asset_graph.get_outgoing_neighbors(dep_node_id);
            for n in neighbors {
              if let Some(AssetGraphNode::Asset(asset)) = asset_graph.get_node(&n) {
                entry_asset_id = Some(asset.id.clone());
                break;
              }
            }
          }
          break;
        }
      }
    }

    let entry_dep = entry_dep.ok_or_else(|| anyhow::anyhow!("missing entry dependency"))?;
    let entry_asset_id = entry_asset_id
      .ok_or_else(|| anyhow::anyhow!("entry dependency did not resolve to an asset"))?;

    let target: Target = entry_dep.target.as_deref().cloned().unwrap_or_default();

    // Build a single monolithic JS bundle.
    let bundle_id = hash_string(format!(
      "bundle:{}{}",
      entry_asset_id,
      target.dist_dir.display()
    ));

    let bundle = Bundle {
      id: bundle_id.clone(),
      public_id: None,
      hash_reference: bundle_id.clone(),
      bundle_type: FileType::Js,
      env: (*target.env).clone(),
      entry_asset_ids: vec![entry_asset_id.clone()],
      main_entry_id: Some(entry_asset_id.clone()),
      needs_stable_name: Some(entry_dep.needs_stable_name),
      bundle_behavior: None,
      is_splittable: Some(false),
      manual_shared_bundle: None,
      name: None,
      pipeline: None,
      target: target.clone(),
    };

    let bundle_group_id = format!("bundle_group:{}{}", target.name, entry_asset_id);

    let bundle_group_node_id =
      bundle_graph.add_bundle_group(bundle_group_id, target, entry_asset_id.clone());
    let bundle_node_id = bundle_graph.add_bundle(bundle);

    // Compute bundle membership by traversing the asset graph from the entry asset.
    let mut stack: Vec<String> = vec![entry_asset_id.clone()];
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();

    let root_node_id = *bundle_graph.get_node_id_by_content_key("@@root").unwrap();

    let entry_dep_node_id = *bundle_graph
      .get_node_id_by_content_key(&entry_dep.id)
      .ok_or_else(|| anyhow::anyhow!("Missing entry dependency node id"))?;

    let entry_asset_node_id = *bundle_graph
      .get_node_id_by_content_key(&entry_asset_id)
      .ok_or_else(|| anyhow::anyhow!("Missing entry asset node id"))?;

    bundle_graph.add_edge(
      &root_node_id,
      &bundle_group_node_id,
      NativeBundleGraphEdgeType::Bundle,
    );
    bundle_graph.add_edge(
      &entry_dep_node_id,
      &bundle_group_node_id,
      NativeBundleGraphEdgeType::Null,
    );
    bundle_graph.add_edge(
      &bundle_group_node_id,
      &bundle_node_id,
      NativeBundleGraphEdgeType::Null,
    );
    bundle_graph.add_edge(
      &bundle_group_node_id,
      &bundle_node_id,
      NativeBundleGraphEdgeType::Bundle,
    );
    bundle_graph.add_edge(
      &bundle_node_id,
      &entry_asset_node_id,
      NativeBundleGraphEdgeType::Null,
    );

    while let Some(asset_id) = stack.pop() {
      if !visited.insert(asset_id.clone()) {
        continue;
      }

      if let Some(node_id) = asset_graph.get_node_id_by_content_key(&asset_id) {
        if let Some(AssetGraphNode::Asset(asset)) = asset_graph.get_node(node_id) {
          if asset.file_type == FileType::Js {
            bundle_graph.add_edge(
              &bundle_node_id,
              node_id,
              NativeBundleGraphEdgeType::Contains,
            );
            bundle_graph.add_edge(&bundle_node_id, node_id, NativeBundleGraphEdgeType::Null);
          }
        }

        for child in asset_graph.get_outgoing_neighbors(node_id) {
          if let Some(child_node) = asset_graph.get_node(&child) {
            match child_node {
              AssetGraphNode::Dependency(dep) => {
                let dep_node_id = *bundle_graph.get_node_id_by_content_key(&dep.id).unwrap();
                bundle_graph.add_edge(
                  &bundle_node_id,
                  &dep_node_id,
                  NativeBundleGraphEdgeType::Contains,
                );
                for dep_child in asset_graph.get_outgoing_neighbors(&child) {
                  if let Some(AssetGraphNode::Asset(a)) = asset_graph.get_node(&dep_child) {
                    stack.push(a.id.clone());
                  }
                }
              }
              AssetGraphNode::Asset(a) => stack.push(a.id.clone()),
              _ => {}
            }
          }
        }
      }
    }

    bundle_graph.add_edge(
      &entry_dep_node_id,
      &entry_asset_node_id,
      NativeBundleGraphEdgeType::References,
    );
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use atlaspack_core::asset_graph::AssetGraph;
  use atlaspack_core::types::{Asset, Dependency, Environment, Target};
  use petgraph::visit::IntoEdgeReferences;

  #[test]
  fn monolithic_bundler_creates_bundle_nodes_and_edges() {
    let mut asset_graph = AssetGraph::new();

    // Create an entry dependency and connect it to an entry asset.
    let target = Target::default();
    let entry_dep = Dependency::entry("entry.js".to_string(), target);
    let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

    let entry_asset = Arc::new(Asset {
      id: "deadbeefdeadbeef".into(),
      file_path: "entry.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let entry_asset_node = asset_graph.add_asset(entry_asset, false);
    asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);

    let bundler = MonolithicBundler::default();
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    let mut bundle_count = 0;
    let mut bundle_group_count = 0;
    for n in bundle_graph.nodes() {
      match n {
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::Bundle(_) => {
          bundle_count += 1
        }
        atlaspack_core::bundle_graph::native_bundle_graph::NativeBundleGraphNode::BundleGroup {
          ..
        } => bundle_group_count += 1,
        _ => {}
      }
    }

    assert_eq!(bundle_count, 1);
    assert_eq!(bundle_group_count, 1);

    // Should include at least one contains edge.
    assert!(
      bundle_graph
        .graph
        .edge_references()
        .any(|e| *e.weight() == NativeBundleGraphEdgeType::Contains)
    );
  }
}
