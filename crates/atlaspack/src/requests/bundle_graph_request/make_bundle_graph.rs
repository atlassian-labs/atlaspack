use std::{
  collections::{HashMap, HashSet},
  path::Path,
  sync::Arc,
};

use atlaspack_core::{
  asset_graph::AssetGraph,
  bundle_graph::{
    AssetRef, BundleDependency, BundleGraph, BundleGraphBundle, BundleGraphEdge, BundleGraphNode,
  },
  types::{Bundle, BundleBehavior, Environment, Target},
};
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeFiltered, EdgeRef, IntoNodeReferences, NodeFiltered},
  Direction,
};
use serde::Serialize;
use tracing::info;

use crate::requests::bundle_graph_request::acyclic_asset_graph::AcyclicAssetGraphNode;

use super::dominator_tree::*;

pub struct MakeBundleGraphParams<'a> {
  pub root_node: NodeIndex,
  pub dominator_tree: &'a StableDiGraph<DominatorTreeNode, DominatorTreeEdge>,
  pub asset_graph: Option<&'a AssetGraph>,
}

pub fn make_bundle_graph(
  MakeBundleGraphParams {
    root_node,
    dominator_tree,
    asset_graph,
  }: MakeBundleGraphParams,
) -> BundleGraph {
  let mut bundle_graph_diagnostics = BundleGraphDiagnostics::default();
  let mut bundle_graph: StableDiGraph<Option<BundleGraphNode>, BundleGraphEdge> = dominator_tree
    .filter_map(
      |_, node| match node {
        AcyclicAssetGraphNode::Root => Some(Some(BundleGraphNode::Root)),
        _ => Some(None),
      },
      |_, _| None,
    );

  struct AddedBundle {
    bundle_dominator_tree_node_index: NodeIndex,
    bundle_node_index: NodeIndex,
  }

  let mut bundles_by_node_index = HashMap::new();
  let mut added_bundles = vec![];

  let mut visited = std::collections::HashSet::new();
  for (_, asset) in dominator_tree.node_references() {
    match asset {
      AcyclicAssetGraphNode::Asset(asset_node) => {
        // println!("Asset: {} {:?}", asset_node.id(), asset_node.file_path());
        let asset_id = asset_node.id();
        if visited.contains(&asset_id) {
          panic!("Asset already visited: {:?}", asset_node.file_path());
        }
        visited.insert(asset_id);
      }
      AcyclicAssetGraphNode::Cycle(assets) => {
        for asset in assets {
          if visited.contains(&asset.id()) {
            panic!("Asset already visited: {:?}", asset.file_path());
          }
          visited.insert(asset.id());
        }
      }
      _ => {}
    }
  }

  let mut assets_discovered = 0;

  let entry_node_indexes = dominator_tree
    .edges(root_node)
    .filter(|edge| match edge.weight() {
      DominatorTreeEdge::EntryAssetRoot(_) => true,
      DominatorTreeEdge::AsyncRoot(_) => true,
      DominatorTreeEdge::SharedBundleRoot => true,
      DominatorTreeEdge::TypeChangeRoot(_) => true,
      DominatorTreeEdge::ImmediateDominator => false,
      DominatorTreeEdge::AssetDependency(_) => false,
      DominatorTreeEdge::AssetAsyncDependency(_) => false,
    })
    .map(|edge| edge.target())
    .collect::<HashSet<_>>();

  for edge in dominator_tree.edges(root_node) {
    let bundle_edge_type = match edge.weight() {
      DominatorTreeEdge::EntryAssetRoot(_) => Some(BundleGraphEdge::RootEntryOf),
      DominatorTreeEdge::AsyncRoot(_) => Some(BundleGraphEdge::RootAsyncBundleOf),
      DominatorTreeEdge::TypeChangeRoot(_) => Some(BundleGraphEdge::RootTypeChangeBundleOf),
      DominatorTreeEdge::SharedBundleRoot => Some(BundleGraphEdge::RootSharedBundleOf),
      DominatorTreeEdge::ImmediateDominator => None,
      DominatorTreeEdge::AssetDependency(_) => None,
      DominatorTreeEdge::AssetAsyncDependency(_) => None,
    };
    let Some(bundle_edge_type) = bundle_edge_type else {
      continue;
    };

    let root_asset = edge.target();
    let mut node_indexes_in_bundle = vec![];
    let mut bundle_assets: StableDiGraph<AssetRef, ()> = StableDiGraph::new();

    let filtered_dominator_tree = EdgeFiltered::from_fn(&dominator_tree, |edge| {
      if entry_node_indexes.contains(&edge.target()) {
        return false;
      }

      match edge.weight() {
        DominatorTreeEdge::ImmediateDominator => true,
        _ => false,
      }
    });
    let filtered_dominator_tree = NodeFiltered::from_fn(&filtered_dominator_tree, |node| {
      node == root_asset || !entry_node_indexes.contains(&node)
    });

    let mut indentation = 0;
    petgraph::visit::depth_first_search(
      &filtered_dominator_tree,
      [root_asset],
      |event| match event {
        petgraph::visit::DfsEvent::Discover(node_index, _) => {
          let _weight = dominator_tree.node_weight(node_index).unwrap();
          // println!("{:indentation$} {}", "", weight);
          indentation += 4;
        }
        petgraph::visit::DfsEvent::Finish(node_index, _) => {
          let node_weight = dominator_tree.node_weight(node_index).unwrap();
          match node_weight {
            AcyclicAssetGraphNode::Asset(asset_node) => {
              node_indexes_in_bundle.push(node_index);

              bundle_assets.add_node(asset_node.clone());
              assets_discovered += 1;
            }
            AcyclicAssetGraphNode::Cycle(assets) => {
              node_indexes_in_bundle.push(node_index);

              for asset in assets {
                node_indexes_in_bundle.push(asset.asset_graph_node_index());
                bundle_assets.add_node(asset.clone());
                assets_discovered += 1;
              }
            }
            _ => {}
          }
          indentation -= 4;
        }
        _ => {}
      },
    );

    let entry_assets = match dominator_tree.node_weight(root_asset).unwrap() {
      AcyclicAssetGraphNode::Asset(asset_node) => vec![asset_node],
      AcyclicAssetGraphNode::Cycle(assets) => assets.iter().collect(),
      _ => panic!("Bundle entry node is not an asset"),
    };

    let entry_asset = entry_assets[0];
    let file_name = entry_asset
      .file_path()
      .file_stem()
      .map(|stem| stem.to_string_lossy().to_string())
      .unwrap_or_else(|| format!("bundle-{}", entry_asset.id()));

    let prefix = match bundle_edge_type {
      BundleGraphEdge::RootEntryOf => "",
      BundleGraphEdge::RootAsyncBundleOf => "async.",
      BundleGraphEdge::RootTypeChangeBundleOf => "",
      BundleGraphEdge::RootSharedBundleOf => "shared.",
      _ => "",
    };

    let bundle_id = format!("bundle(entry={})", entry_asset.id());
    let bundle_name = Some(format!(
      "{}{}.{}.{}",
      prefix,
      file_name,
      entry_asset.id().to_string(),
      entry_asset.file_type().extension()
    ));

    bundle_graph_diagnostics.bundles.push(BundleDiagnostics {
      bundle_id: bundle_id.clone(),
      bundle_name: bundle_name.clone().unwrap(),
      assets: bundle_assets
        .node_weights()
        .map(|asset| asset.file_path().to_string_lossy().to_string())
        .collect(),
      dependencies: vec![],
    });

    let bundle_graph_bundle = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: entry_asset.file_type().clone(),
        // This isn't right ;
        entry_asset_ids: entry_assets.iter().map(|asset| asset.id()).collect(),
        env: Environment::default(),
        hash_reference: "".to_string(),
        id: bundle_id,
        is_splittable: true,
        main_entry_id: Some(entry_asset.id()),
        manual_shared_bundle: None,
        name: bundle_name,
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: bundle_assets,
    };

    let bundle_node_index =
      bundle_graph.add_node(Some(BundleGraphNode::Bundle(Arc::new(bundle_graph_bundle))));
    bundle_graph.add_edge(root_node, bundle_node_index, bundle_edge_type.clone());

    // println!(
    //   "  >> added bundle {} (bundle {:?})",
    //   bundle_node_index.index(),
    //   bundle_edge_type
    // );
    added_bundles.push(AddedBundle {
      bundle_dominator_tree_node_index: root_asset,
      bundle_node_index,
    });

    for node_index in node_indexes_in_bundle {
      bundles_by_node_index.insert(node_index, bundle_node_index);
    }
  }

  info!("Discovered {assets_discovered} assets");

  for (node_index, node_weight) in dominator_tree.node_references() {
    match node_weight {
      AcyclicAssetGraphNode::Asset(_asset_node) => {
        if !bundles_by_node_index.contains_key(&node_index) {
          tracing::error!(
            "Asset is not in any bundle:\n --> path={:#?} asset_graph_node={:?}",
            dominator_tree.node_weight(node_index).unwrap().file_path(),
            asset_graph.unwrap().graph.node_weight(node_index)
          );

          // has path to root?
          let has_path_to_root = petgraph::algo::has_path_connecting(
            &EdgeFiltered::from_fn(&dominator_tree, |edge| {
              matches!(edge.weight(), DominatorTreeEdge::ImmediateDominator)
            }),
            root_node,
            node_index,
            None,
          );

          if !has_path_to_root {
            tracing::error!("Asset is not connected to root: {:?}", node_index);
          }

          if let Some(asset_graph) = asset_graph {
            debug_node_chain(&dominator_tree, node_index);
            debug_asset_dependency_chain(
              asset_graph,
              dominator_tree
                .node_weight(node_index)
                .unwrap()
                .file_path()
                .unwrap(),
            );
          }
        }
      }
      AcyclicAssetGraphNode::Cycle(assets) => {
        for asset in assets {
          if !bundles_by_node_index.contains_key(&asset.asset_graph_node_index()) {
            tracing::error!(
              "Asset is not in any bundle:\n --> path={:#?}",
              asset.file_path()
            );
          }
        }
      }
      _ => {}
    }
  }

  // Link the bundles together
  for AddedBundle {
    bundle_dominator_tree_node_index,
    bundle_node_index,
  } in added_bundles.iter()
  {
    let filtered_dominator_tree =
      EdgeFiltered::from_fn(&dominator_tree, |edge| match edge.weight() {
        DominatorTreeEdge::ImmediateDominator => true,
        _ => false,
      });

    petgraph::visit::depth_first_search(
      &filtered_dominator_tree,
      [*bundle_dominator_tree_node_index],
      |event| match event {
        petgraph::visit::DfsEvent::Discover(node_index, _) => {
          // println!(
          //   "Discovering node: {}",
          //   dominator_tree.node_weight(node_index).unwrap()
          // );
          let source_bundle_index = bundles_by_node_index[&node_index];
          let source_bundle = bundle_graph
            .node_weight(source_bundle_index)
            .unwrap()
            .as_ref()
            .unwrap()
            .as_bundle()
            .unwrap()
            .clone();

          for edge in dominator_tree.edges_directed(node_index, Direction::Outgoing) {
            // println!("  Discovering edge: {}", edge.weight());
            match edge.weight() {
              DominatorTreeEdge::ImmediateDominator => {}
              DominatorTreeEdge::AssetDependency(edge_weight) => {
                let target = edge.target();
                let target_bundle = bundles_by_node_index[&target];
                //   tracing::error!(
                //     "Asset is not in any bundle:\n --> path={:#?}",
                //     dominator_tree.node_weight(target).unwrap().file_path()
                //   );
                //   continue;
                // };

                if target_bundle == *bundle_node_index {
                  continue;
                }

                bundle_graph_diagnostics
                  .bundles
                  .iter_mut()
                  .find(|diagnostics| diagnostics.bundle_id == source_bundle.bundle.id)
                  .as_mut()
                  .unwrap()
                  .dependencies
                  .push((
                    "sync".to_string(),
                    bundle_graph
                      .node_weight(target_bundle)
                      .unwrap()
                      .as_ref()
                      .unwrap()
                      .as_bundle()
                      .unwrap()
                      .bundle
                      .id
                      .clone(),
                  ));
                bundle_graph.add_edge(
                  *bundle_dominator_tree_node_index,
                  target_bundle,
                  // TODO: Maintain the reason this edge is here, e.g.: carry over the source, target, dependency etc
                  BundleGraphEdge::BundleSyncLoads(BundleDependency::new(
                    edge_weight,
                    edge_weight.target_node_index(),
                  )),
                );
              }
              DominatorTreeEdge::AssetAsyncDependency(edge_weight) => {
                let target = edge.target();
                let target_bundle = bundles_by_node_index[&target];

                // let source_weight = dominator_tree.node_weight(node_index).unwrap();
                // let target_weight = dominator_tree.node_weight(target).unwrap();

                bundle_graph_diagnostics
                  .bundles
                  .iter_mut()
                  .find(|diagnostics| diagnostics.bundle_id == source_bundle.bundle.id)
                  .as_deref_mut()
                  .unwrap()
                  .dependencies
                  .push((
                    "async".to_string(),
                    bundle_graph
                      .node_weight(target_bundle)
                      .unwrap()
                      .as_ref()
                      .unwrap()
                      .as_bundle()
                      .unwrap()
                      .bundle
                      .id
                      .clone(),
                  ));
                bundle_graph.add_edge(
                  *bundle_node_index,
                  target_bundle,
                  // TODO: Maintain the reason this edge is here, e.g.: carry over the source, target, dependency etc
                  // TODO: We need to keep track of the module ID the dependency resolves into.
                  BundleGraphEdge::BundleAsyncLoads(BundleDependency::new(
                    edge_weight,
                    edge_weight.target_node_index(),
                  )),
                );
              }
              DominatorTreeEdge::EntryAssetRoot(_) => unreachable!(),
              DominatorTreeEdge::AsyncRoot(_) => unreachable!(),
              DominatorTreeEdge::SharedBundleRoot => unreachable!(),
              DominatorTreeEdge::TypeChangeRoot(_) => unreachable!(),
            }
          }
        }
        _ => {}
      },
    );
  }

  std::fs::write(
    "bundle-graph-diagnostics.json",
    serde_json::to_string_pretty(&bundle_graph_diagnostics).unwrap(),
  )
  .unwrap();

  let bundle_graph = bundle_graph.filter_map(|_, node| node.clone(), |_, edge| Some(edge.clone()));

  BundleGraph::build_from(root_node, bundle_graph)
}

#[derive(Serialize, Default)]
pub struct BundleGraphDiagnostics {
  bundles: Vec<BundleDiagnostics>,
}

#[derive(Serialize)]
pub struct BundleDiagnostics {
  bundle_id: String,
  bundle_name: String,
  assets: Vec<String>,
  dependencies: Vec<(String, String)>,
}

pub fn debug_node_chain(
  dominator_tree: &StableDiGraph<DominatorTreeNode, DominatorTreeEdge>,
  node_index: NodeIndex,
) {
  println!("DOMINATOR TREE - Dependency chain up from {node_index:?}");
  let mut asset = node_index;
  let mut indentation = 2;

  for _ in 0..50 {
    dominator_tree
      .edges_directed(asset, Direction::Incoming)
      .for_each(|edge| {
        let source = edge.source();
        let source_weight = dominator_tree.node_weight(source).unwrap();
        println!(
          "{:indentation$} <-- {} -- {}",
          "",
          edge.weight(),
          source_weight,
        );
        asset = source;
      });

    indentation += 2;
  }
}

pub fn debug_asset_dependency_chain(asset_graph: &AssetGraph, file_path: &Path) {
  info!("Debugging asset dependency chain for: {:#?}", file_path);

  // find the asset node
  let (asset, _) = asset_graph
    .graph
    .node_references()
    .find(|(_, node)| {
      node
        .as_asset_node()
        .map(|asset_node| asset_node.asset.file_path == file_path)
        .unwrap_or(false)
    })
    .expect("Asset not found");

  let root = asset_graph.root_node();
  // Find all paths from the root to the asset
  let paths = petgraph::algo::all_simple_paths::<Vec<_>, _, std::hash::RandomState>(
    &asset_graph.graph,
    asset,
    root,
    0,
    None,
  );

  println!("Path into {file_path:?}:");

  for path in paths.take(3) {
    let mut identation = 0;

    for node in path {
      let node_weight = asset_graph.graph.node_weight(node).unwrap();
      println!("{:identation$} <-- {}", "", node_weight);

      identation += 2;
    }
  }

  println!("Dependency chain up from {file_path:?}");
  let mut asset = asset;
  let mut indentation = 2;

  for _ in 0..50 {
    asset_graph
      .graph
      .edges_directed(asset, Direction::Incoming)
      .for_each(|edge| {
        let source = edge.source();
        let source_weight = asset_graph.graph.node_weight(source).unwrap();
        println!("{:indentation$} <-- {}", "", source_weight,);
        asset = source;
      });

    indentation += 2;
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_core::bundle_graph::BundleRef;

  use crate::requests::bundle_graph_request::{
    acyclic_asset_graph::remove_cycles, asset_graph_builder::simplified_asset_graph_builder,
  };

  use super::*;

  fn get_bundle_by_name(bundle_graph: &BundleGraph, name: &str) -> Option<BundleRef> {
    bundle_graph.bundles().find(|bundle| {
      bundle
        .bundle_graph_bundle()
        .bundle
        .name
        .as_ref()
        .unwrap()
        .contains(name)
    })
  }

  #[test]
  fn test_make_bundle_graph_with_nested_dependency() {
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let b = graph.asset("b.js");
    let c = graph.asset("c.js");
    let d = graph.asset("d.js");
    graph.async_dependency(a, b);
    graph.async_dependency(a, c);
    graph.sync_dependency(b, d);
    graph.sync_dependency(c, d);

    let graph = graph.build();

    let (root_node, graph) = remove_cycles(&graph);
    let dominator_tree = build_dominator_tree(&graph, root_node, false);

    let bundle_graph = make_bundle_graph(MakeBundleGraphParams {
      root_node: root_node,
      dominator_tree: &dominator_tree,
      asset_graph: None,
    });

    assert_eq!(bundle_graph.num_bundles(), 4);

    let bundle_a = get_bundle_by_name(&bundle_graph, "a.js").unwrap();
    let bundle_b = get_bundle_by_name(&bundle_graph, "b.js").unwrap();
    let bundle_c = get_bundle_by_name(&bundle_graph, "c.js").unwrap();
    let bundle_d = get_bundle_by_name(&bundle_graph, "d.js").unwrap();

    assert_eq!(bundle_a.num_assets(), 1);
    assert_eq!(bundle_b.num_assets(), 1);
    assert_eq!(bundle_c.num_assets(), 1);
    assert_eq!(bundle_d.num_assets(), 1);

    let mut bundle_a_assets = bundle_a
      .assets()
      .map(|asset| asset.file_path().to_string_lossy().to_string())
      .collect::<Vec<_>>();
    bundle_a_assets.sort();
    assert_eq!(bundle_a_assets, vec!["a.js"]);

    let mut bundle_b_assets = bundle_b
      .assets()
      .map(|asset| asset.file_path().to_string_lossy().to_string())
      .collect::<Vec<_>>();
    bundle_b_assets.sort();
    assert_eq!(bundle_b_assets, vec!["b.js"]);

    let mut bundle_c_assets = bundle_c
      .assets()
      .map(|asset| asset.file_path().to_string_lossy().to_string())
      .collect::<Vec<_>>();
    bundle_c_assets.sort();
    assert_eq!(bundle_c_assets, vec!["c.js"]);

    let mut bundle_d_assets = bundle_d
      .assets()
      .map(|asset| asset.file_path().to_string_lossy().to_string())
      .collect::<Vec<_>>();
    bundle_d_assets.sort();
    assert_eq!(bundle_d_assets, vec!["d.js"]);
  }
}
