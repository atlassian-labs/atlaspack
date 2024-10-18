//! This module contains a few abstract bundling algorithms to bundle assets
//! together.

use std::collections::HashMap;

use petgraph::graph::DiGraph;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Direction};

type BundlerAssetId = usize;

/// This is a simplified structure representing an asset that is being bundled.
///
/// An asset is a file such as a JavaScript file, with a set of dependencies.
#[derive(Debug)]
pub struct BundlerAsset(#[allow(unused)] BundlerAssetId);

#[derive(Debug, Clone, PartialEq)]
pub enum BundlerDependencyType {
  Synchronous,
  Asynchronous,
}

pub type BundlerAssetGraph = DiGraph<BundlerAsset, BundlerDependencyType>;

pub struct BundlerParams<'a> {
  /// The asset graph containing minimal required asset information
  asset_graph: &'a BundlerAssetGraph,
  /// The node indexes of entry-points to the asset graph
  entry_points: &'a [NodeIndex],
}

#[derive(Debug)]
pub struct Bundle {
  assets: Vec<BundlerAssetId>,
}

pub type BundleGraph = DiGraph<Bundle, ()>;

pub struct BundlerResult {
  bundles: BundleGraph,
}

impl BundlerResult {
  pub fn print_dot(&self, asset_graph: &BundlerAssetGraph) -> String {
    let mut output = "digraph {\n".to_string();

    let mut bundles_by_asset_id: HashMap<BundlerAssetId, Vec<NodeIndex>> = HashMap::new();
    for bundle_index in self.bundles.node_indices() {
      let assets = &self.bundles[bundle_index].assets;
      for asset in assets {
        bundles_by_asset_id
          .entry(*asset)
          .or_default()
          .push(bundle_index);
      }
    }

    for bundle_index in self.bundles.node_indices() {
      let bundle_id = bundle_index.index();
      output += &format!("    subgraph cluster_bundle_{} {{\n", bundle_id);
      output += &format!("        label = \"Bundle {bundle_id}\";\n");
      let assets = &self.bundles[bundle_index].assets;
      for asset in assets {
        let asset_node_name = format!("bundle_{bundle_id}_asset_{asset}");
        output += &format!("          {asset_node_name};\n");

        for edge in asset_graph.edges_directed(NodeIndex::new(*asset), Direction::Outgoing) {
          let target_asset = edge.target().index();

          if assets.contains(&target_asset) {
            let target_asset_node_name = format!("bundle_{bundle_id}_asset_{target_asset}");
            output += &format!("        {asset_node_name} -> {target_asset_node_name};\n");
          } else {
            // The asset is not on this bundle. There are two options for this to happen:
            //
            // * It's on a parent, this is an async bundle
            // * It's on a shared bundle
            for bundle_index in bundles_by_asset_id.get(&target_asset).unwrap() {
              let bundle_id = bundle_index.index();
              let target_asset_node_name = format!("bundle_{bundle_id}_asset_{target_asset}");
              output += &format!("        {asset_node_name} -> {target_asset_node_name};\n");
            }
          };
        }
      }
      output += "    }\n"
    }

    output += "}";
    output
  }
}

impl BundlerResult {
  pub fn into_bundles(self) -> Vec<Bundle> {
    self
      .bundles
      .into_nodes_edges()
      .0
      .into_iter()
      .map(|node| node.weight)
      .collect()
  }
}

#[derive(Debug)]
pub enum BundlerError {}

/// A bundler that puts all assets into a single output bundle.
pub mod mono_chunk_bundler {
  use petgraph::graph::DiGraph;

  use super::*;

  #[derive(Default)]
  pub struct MonoChunkBundler {}

  impl MonoChunkBundler {
    pub fn bundle(&self, params: BundlerParams) -> Result<BundlerResult, BundlerError> {
      let main_bundle = Bundle {
        assets: params
          .asset_graph
          .node_indices()
          .map(|node| node.index())
          .collect(),
      };

      let mut bundles = DiGraph::new();
      bundles.add_node(main_bundle);
      let result = BundlerResult { bundles };

      Ok(result)
    }
  }

  #[cfg(test)]
  mod test {
    use petgraph::graph::NodeIndex;

    use super::*;

    #[test]
    fn test_mono_chunk_bundler() {
      /*

      ASSET_1
      ASSET_2 ---> ASSET_3

      result in 1 bundle for these two graphs

             */
      let mut asset_graph = BundlerAssetGraph::new();
      let asset1 = asset_graph.add_node(BundlerAsset(0)).index();
      let asset2 = asset_graph.add_node(BundlerAsset(1)).index();
      let asset3 = asset_graph.add_node(BundlerAsset(2)).index();
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset3),
        BundlerDependencyType::Synchronous,
      );

      let bundler = MonoChunkBundler::default();
      let params = BundlerParams {
        asset_graph: &asset_graph,
        entry_points: &[],
      };
      let result = bundler.bundle(params).unwrap();
      let bundles: Vec<Bundle> = result.into_bundles();
      assert_eq!(bundles.len(), 1);
      assert_eq!(bundles[0].assets.len(), 3);
      assert_eq!(bundles[0].assets, vec![asset1, asset2, asset3]);
    }
  }
}

/// A bundler that will bundle assets of connected sub-trees into separate
/// bundles, but still ignore dependency types between bundles.
pub mod simple_subtree_bundler {
  use petgraph::graph::DiGraph;
  use petgraph::visit::Dfs;

  use super::*;

  #[derive(Default)]
  pub struct SimpleSubtreeBundler {}

  impl SimpleSubtreeBundler {
    pub fn bundle(&self, params: BundlerParams) -> Result<BundlerResult, BundlerError> {
      let mut bundles = DiGraph::new();
      for entry_point in params.entry_points {
        let mut bundle = Bundle {
          assets: vec![entry_point.index()],
        };

        let mut dfs = Dfs::new(&params.asset_graph, *entry_point);
        while let Some(node) = dfs.next(&params.asset_graph) {
          if node == *entry_point {
            continue;
          }
          bundle.assets.push(node.index());
        }

        bundles.add_node(bundle);
      }

      Ok(BundlerResult { bundles })
    }
  }

  #[cfg(test)]
  mod test {
    use petgraph::{graph::NodeIndex, Graph};

    use super::*;

    #[test]
    fn test_simple_bundler_on_simple_graph() {
      /*

      ASSET_1
      ASSET_2 ---> ASSET_3

      result in 2 bundles for these two graphs

             */
      let mut asset_graph = Graph::new();
      let asset1 = asset_graph.add_node(BundlerAsset(0)).index();
      let asset2 = asset_graph.add_node(BundlerAsset(1)).index();
      let asset3 = asset_graph.add_node(BundlerAsset(2)).index();
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset3),
        BundlerDependencyType::Synchronous,
      );

      let bundler = SimpleSubtreeBundler::default();
      let params = BundlerParams {
        asset_graph: &asset_graph,
        entry_points: &[NodeIndex::new(asset1), NodeIndex::new(asset2)],
      };
      let result = bundler.bundle(params).unwrap();
      let mut bundles: Vec<Bundle> = result.into_bundles();
      bundles.sort_by(|bundle1, bundle2| bundle1.assets[0].cmp(&bundle2.assets[0]));
      assert_eq!(bundles.len(), 2);
      assert_eq!(bundles[0].assets.len(), 1);
      assert_eq!(bundles[1].assets.len(), 2);
      assert_eq!(bundles[0].assets, vec![asset1]);
      assert_eq!(bundles[1].assets, vec![asset2, asset3]);
    }

    #[test]
    fn test_simple_bundler_on_nested_graph() {
      /*

         /------------------------------------------------> ASSET_6
      ASSET_1 ------------------------- async --\             /
      ASSET_2 ------> ASSET_3 --- async ----> ASSET_4 ----> ASSET_5

      */
      let mut asset_graph = Graph::new();
      let asset1 = asset_graph.add_node(BundlerAsset(0)).index();
      let asset2 = asset_graph.add_node(BundlerAsset(1)).index();
      let asset3 = asset_graph.add_node(BundlerAsset(2)).index();
      let asset4 = asset_graph.add_node(BundlerAsset(3)).index();
      let asset5 = asset_graph.add_node(BundlerAsset(4)).index();
      let asset6 = asset_graph.add_node(BundlerAsset(5)).index();

      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset3),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset4),
        NodeIndex::new(asset5),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset5),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset3),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );

      let bundler = SimpleSubtreeBundler::default();
      let params = BundlerParams {
        asset_graph: &asset_graph,
        entry_points: &[NodeIndex::new(asset1), NodeIndex::new(asset2)],
      };
      let result = bundler.bundle(params).unwrap();
      let mut bundles = result.into_bundles();
      bundles.sort_by(|bundle1, bundle2| bundle1.assets[0].cmp(&bundle2.assets[0]));
      assert_eq!(bundles.len(), 2);
      assert_eq!(bundles[0].assets.len(), 4);
      assert_eq!(bundles[1].assets.len(), 5);
      assert_eq!(bundles[0].assets, vec![asset1, asset6, asset4, asset5]);
      assert_eq!(
        bundles[1].assets,
        vec![asset2, asset3, asset4, asset5, asset6]
      );
    }
  }
}

/// A bundler that will bundle assets of connected sub-trees into separate
/// bundles. For all async import boundaries, a new bundle will be created.
///
/// Async bundles that are equal will be deduplicated.
///
/// This bundler will duplicate assets into async bundles, regardless of
/// whether they are already in all the requesting sync bundles, if they
/// happen to appear on both edges. See the unit-test to understand what
/// this means.
pub mod subtree_bundler {
  use std::collections::BTreeMap;

  use petgraph::graph::DiGraph;
  use petgraph::prelude::EdgeRef;
  use petgraph::Direction;

  use super::*;

  #[derive(Default)]
  pub struct SubtreeBundler {}

  impl SubtreeBundler {
    pub fn bundle(&self, params: BundlerParams) -> Result<BundlerResult, BundlerError> {
      let mut bundles = DiGraph::new();
      let mut handled_entry_points = BTreeMap::new();
      let mut stack: Vec<_> = params
        .entry_points
        .iter()
        .cloned()
        .map(|index| (index, None))
        .collect();

      while let Some((entry_point, parent_entry_point)) = stack.pop() {
        let bundle_index = if let Some(bundle_index) = handled_entry_points.get(&entry_point) {
          *bundle_index
        } else {
          let mut bundle = Bundle {
            assets: vec![entry_point.index()],
          };

          let mut current_node_stack = vec![entry_point];
          while let Some(current_parent) = current_node_stack.pop() {
            for edge in params
              .asset_graph
              .edges_directed(current_parent, Direction::Outgoing)
            {
              let node = edge.target();
              let is_asynchronous =
                is_asynchronous_dependency(&params.asset_graph, current_parent, node);

              if is_asynchronous {
                stack.push((node, Some(entry_point)));
              } else {
                bundle.assets.push(node.index());
                current_node_stack.push(node);
              }
            }
          }
          let bundle_index = bundles.add_node(bundle);
          handled_entry_points.insert(entry_point, bundle_index);
          bundle_index
        };

        if let Some(parent_bundle_index) = parent_entry_point
          .and_then(|parent_asset_index| handled_entry_points.get(&parent_asset_index))
        {
          bundles.add_edge(*parent_bundle_index, bundle_index, ());
        }
      }

      Ok(BundlerResult { bundles })
    }
  }

  fn is_asynchronous_dependency(
    asset_graph: &BundlerAssetGraph,
    entry_point: NodeIndex,
    node: NodeIndex,
  ) -> bool {
    let edges = asset_graph.edges_directed(entry_point, Direction::Outgoing);
    let is_asynchronous = edges
      .filter(|edge| edge.source() == entry_point && edge.target() == node)
      .all(|edge| edge.weight() == &BundlerDependencyType::Asynchronous);
    is_asynchronous
  }

  #[cfg(test)]
  mod test {
    use petgraph::{graph::NodeIndex, Graph};

    use super::*;

    #[test]
    fn test_simple_bundler() {
      /*

         /------------------------------------------------> ASSET_6
      ASSET_1 ------------------------- async --\             /
      ASSET_2 ------> ASSET_3 --- async ----> ASSET_4 ----> ASSET_5

      result in 3 bundles for:

      * asset1, asset6
      * asset2, asset3
      * asset4, asset5, asset6

      Note asset6 is duplicated in the synchronous entry and ansynchronous
      bundles

             */
      let mut asset_graph = Graph::new();
      let asset1 = asset_graph.add_node(BundlerAsset(0)).index();
      let asset2 = asset_graph.add_node(BundlerAsset(1)).index();
      let asset3 = asset_graph.add_node(BundlerAsset(2)).index();
      let asset4 = asset_graph.add_node(BundlerAsset(3)).index();
      let asset5 = asset_graph.add_node(BundlerAsset(4)).index();
      let asset6 = asset_graph.add_node(BundlerAsset(5)).index();

      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset3),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset4),
        NodeIndex::new(asset5),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset5),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset3),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );

      let edges: Vec<_> = asset_graph
        .edges_connecting(NodeIndex::new(asset1), NodeIndex::new(asset6))
        .collect();
      assert_eq!(edges.len(), 1);

      let bundler = SubtreeBundler::default();
      let params = BundlerParams {
        asset_graph: &asset_graph,
        entry_points: &[NodeIndex::new(asset1), NodeIndex::new(asset2)],
      };
      let result = bundler.bundle(params).unwrap();
      let mut bundles = result.into_bundles();
      bundles.sort_by(|bundle1, bundle2| bundle1.assets[0].cmp(&bundle2.assets[0]));
      assert_eq!(bundles.len(), 3);
      assert_eq!(bundles[0].assets, vec![asset1, asset6]);
      assert_eq!(bundles[1].assets, vec![asset2, asset3]);
      assert_eq!(bundles[2].assets, vec![asset4, asset5, asset6]);
    }
  }
}

/// A bundler that will bundle assets of connected subtrees into separate
/// bundles. For all async import boundaries, a new bundle will be created.
///
/// Async bundles that are equal will be deduplicated.
///
/// If a child asset is used on its parent bundles, it will always be hoisted
/// up.
pub mod deduplicating_subtree_bundler {
  use std::collections::HashMap;

  use petgraph::prelude::EdgeRef;
  use petgraph::visit::Topo;
  use petgraph::Direction;

  use super::*;

  #[derive(Default)]
  pub struct DeduplicatingSubtreeBundler {}

  impl DeduplicatingSubtreeBundler {
    pub fn bundle(&self, params: BundlerParams) -> Result<BundlerResult, BundlerError> {
      let subtree_bundler = subtree_bundler::SubtreeBundler::default();
      let BundlerResult { mut bundles } = subtree_bundler.bundle(params)?;

      let mut bundle_node_index_by_asset_id: HashMap<BundlerAssetId, Vec<NodeIndex>> =
        HashMap::new();

      for bundle_index in bundles.node_indices() {
        let bundle = bundles.node_weight(bundle_index).unwrap();
        for asset in &bundle.assets {
          bundle_node_index_by_asset_id
            .entry(*asset)
            .or_default()
            .push(bundle_index);
        }
      }

      let mut topological = Topo::new(&bundles);
      while let Some(bundle_index) = topological.next(&bundles) {
        let incoming_dependencies: Vec<_> = bundles
          .edges_directed(bundle_index, Direction::Incoming)
          .map(|edge| edge.source())
          .collect();
        let bundle = bundles.node_weight_mut(bundle_index).unwrap();

        // println!("Visit {bundle:?}");
        let mut new_assets = vec![];
        for asset in &bundle.assets {
          let bundles_with_asset = bundle_node_index_by_asset_id.get(asset).unwrap();
          // println!("Bundles with asset: {bundles_with_asset:?} Incoming dependencies: {incoming_dependencies:?}");
          //
          // This is not correct because deduplication will not work if there are two bundles
          // between the usage and declaration.
          let can_skip_asset = !incoming_dependencies.is_empty()
            && incoming_dependencies
              .iter()
              .all(|source_bundle_index| bundles_with_asset.contains(&source_bundle_index));
          if can_skip_asset {
            continue;
          }
          new_assets.push(*asset);
        }

        bundle.assets = new_assets;
      }

      Ok(BundlerResult { bundles })
    }
  }

  #[cfg(test)]
  mod test {
    use petgraph::{graph::NodeIndex, Graph};

    use super::*;

    #[test]
    fn test_deduplicating_subtree_bundler() {
      /*

         /------------------------------------------------> ASSET_6 <-------\
      ASSET_1 ------------------------- async --\             /             |
      ASSET_2 ------> ASSET_3 --- async ----> ASSET_4 ----> ASSET_5         |
         \------------------------------------------------------------------/

      result in 3 bundles for:

      * asset1, asset6
      * asset2, asset3
      * asset4, asset5, asset6

      Note asset6 is duplicated in the synchronous entry and ansynchronous
      bundles

             */
      let mut asset_graph = Graph::new();
      let asset1 = asset_graph.add_node(BundlerAsset(0)).index();
      let asset2 = asset_graph.add_node(BundlerAsset(1)).index();
      let asset3 = asset_graph.add_node(BundlerAsset(2)).index();
      let asset4 = asset_graph.add_node(BundlerAsset(3)).index();
      let asset5 = asset_graph.add_node(BundlerAsset(4)).index();
      let asset6 = asset_graph.add_node(BundlerAsset(5)).index();

      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset3),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset4),
        NodeIndex::new(asset5),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset5),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset3),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );

      let dot = petgraph::dot::Dot::new(&asset_graph);
      println!("{:?}", dot);

      let edges: Vec<_> = asset_graph
        .edges_connecting(NodeIndex::new(asset1), NodeIndex::new(asset6))
        .collect();
      assert_eq!(edges.len(), 1);

      let bundler = DeduplicatingSubtreeBundler::default();
      let params = BundlerParams {
        asset_graph: &asset_graph,
        entry_points: &[NodeIndex::new(asset1), NodeIndex::new(asset2)],
      };
      let result = bundler.bundle(params).unwrap();
      println!("{}", result.print_dot(&asset_graph));
      let mut bundles = result.into_bundles();
      assert_eq!(bundles.len(), 3);
      bundles.sort_by(|bundle1, bundle2| bundle1.assets[0].cmp(&bundle2.assets[0]));
      assert_eq!(bundles[0].assets, vec![asset1, asset6]);
      assert_eq!(bundles[1].assets, vec![asset2, asset3, asset6]);
      assert_eq!(bundles[2].assets, vec![asset4, asset5]);
    }
  }
}

pub mod dominators_bundler {
  use std::collections::{HashMap, HashSet};
  use std::fmt::{Debug, Formatter};

  use petgraph::dot::Dot;
  use petgraph::prelude::EdgeRef;

  use super::*;

  #[derive(Clone, Debug)]
  enum RootGraphNode {
    Root,
    Asset(NodeIndex),
  }

  struct DominatorTreeNode {
    root_weight: RootGraphNode,
  }

  impl Debug for DominatorTreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      match self.root_weight {
        RootGraphNode::Root => f.write_str("root"),
        RootGraphNode::Asset(asset) => f.write_fmt(format_args!("asset_{}", asset.index())),
      }
    }
  }

  #[derive(Default)]
  pub struct DominatorsBundler {}

  impl DominatorsBundler {
    pub fn bundle(&self, params: BundlerParams) -> Result<BundlerResult, BundlerError> {
      let (root_graph, root_index) = build_root_graph(&params);

      let dominators = petgraph::algo::dominators::simple_fast(&root_graph, root_index);
      // Copy root graph into dominator graph; indexes will be the same
      let mut dominator_tree = DiGraph::new();
      for node in root_graph.raw_nodes() {
        dominator_tree.add_node(DominatorTreeNode {
          root_weight: node.weight.clone(),
        });
      }

      // Build dominator tree
      let mut visit_stack = vec![root_index];
      let mut seen = HashSet::new();
      while let Some(start_node) = visit_stack.pop() {
        if seen.contains(&start_node) {
          continue;
        }
        seen.insert(start_node);
        let dominators = dominators.immediately_dominated_by(start_node);
        println!("entry = {start_node:?}");

        for node in dominators {
          println!("dominated = {node:?}");
          dominator_tree.add_edge(start_node, node, ());
          visit_stack.push(node);
        }
      }
      let dominator_tree_dot = Dot::new(&dominator_tree);
      println!("{:?}", dominator_tree_dot);

      // Now each child of the root node on the dominator tree is a "chunk".
      // We can treat each chunk as a separate bundle

      let mut bundles = BundleGraph::new();
      for edge in dominator_tree.edges_directed(root_index, Direction::Outgoing) {
        let DominatorTreeNode { root_weight } = dominator_tree.node_weight(edge.target()).unwrap();
        // this is irrefutable since there is only one root
        let RootGraphNode::Asset(asset_index) = root_weight else {
          continue;
        };
        let mut bundle = Bundle {
          assets: vec![asset_index.index()],
        };
        let mut stack = vec![edge.target()];
        while let Some(current_node) = stack.pop() {
          for edge in dominator_tree.edges_directed(current_node, Direction::Outgoing) {
            let DominatorTreeNode { root_weight } =
              dominator_tree.node_weight(edge.target()).unwrap();
            // this is irrefutable since there is only one root
            let RootGraphNode::Asset(asset_index) = root_weight else {
              continue;
            };
            bundle.assets.push(asset_index.index());
          }
        }
        bundles.add_node(bundle);
      }
      let bundles_dot = petgraph::dot::Dot::new(&bundles);
      println!("{:?}", bundles_dot);

      Ok(BundlerResult { bundles })
    }
  }

  /// Convert asset graph without root nodes to asset graph with a root node
  fn build_root_graph(
    params: &BundlerParams,
  ) -> (
    DiGraph<RootGraphNode, Option<BundlerDependencyType>>,
    NodeIndex,
  ) {
    let asset_graph = params.asset_graph;
    let mut root_graph: DiGraph<RootGraphNode, Option<BundlerDependencyType>> = DiGraph::new();
    let root_index = root_graph.add_node(RootGraphNode::Root);
    let mut asset_graph_node_index_to_root_index = HashMap::new();

    for node in asset_graph.node_indices() {
      let root_graph_index = root_graph.add_node(RootGraphNode::Asset(node));
      asset_graph_node_index_to_root_index.insert(node, root_graph_index);
    }

    for node in asset_graph.node_indices() {
      let root_graph_index = asset_graph_node_index_to_root_index.get(&node).unwrap();
      for edge in asset_graph.edges_directed(node, Direction::Outgoing) {
        let target_index = edge.target();
        let target_index = asset_graph_node_index_to_root_index
          .get(&target_index)
          .unwrap();
        root_graph.add_edge(
          *root_graph_index,
          *target_index,
          Some(edge.weight().clone()),
        );
      }
    }

    for entry in params.entry_points {
      let entry_index = asset_graph_node_index_to_root_index.get(entry).unwrap();
      root_graph.add_edge(root_index, *entry_index, None);
    }

    (root_graph, root_index)
  }

  #[cfg(test)]
  mod test {
    use petgraph::{graph::NodeIndex, Graph};

    use super::*;

    #[test]
    fn test_dominators_bundler() {
      /*

         /------------------------------------------------> ASSET_6 <-------\
      ASSET_1 ------------------------- async --\             /             |
      ASSET_2 ------> ASSET_3 --- async ----> ASSET_4 ----> ASSET_5         |
         \------------------------------------------------------------------/

      result in 4 bundles for:

      * asset1
      * asset2, asset3
      * asset4, asset5
      * asset6

      Note asset6 is split into its own bundle, despite being a synchronous dependency of 1, 2 and 5
      This is effectively an "intermediary chunk" and this relationship needs to be handled by the
      bundler before flushing scripts to a browser or writing files to disk.

             */
      let mut asset_graph = Graph::new();
      let asset1 = asset_graph.add_node(BundlerAsset(0)).index();
      let asset2 = asset_graph.add_node(BundlerAsset(1)).index();
      let asset3 = asset_graph.add_node(BundlerAsset(2)).index();
      let asset4 = asset_graph.add_node(BundlerAsset(3)).index();
      let asset5 = asset_graph.add_node(BundlerAsset(4)).index();
      let asset6 = asset_graph.add_node(BundlerAsset(5)).index();

      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset2),
        NodeIndex::new(asset3),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset4),
        NodeIndex::new(asset5),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset5),
        NodeIndex::new(asset6),
        BundlerDependencyType::Synchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset3),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );
      asset_graph.add_edge(
        NodeIndex::new(asset1),
        NodeIndex::new(asset4),
        BundlerDependencyType::Asynchronous,
      );

      let dot = petgraph::dot::Dot::new(&asset_graph);
      println!("{:?}", dot);

      let edges: Vec<_> = asset_graph
        .edges_connecting(NodeIndex::new(asset1), NodeIndex::new(asset6))
        .collect();
      assert_eq!(edges.len(), 1);

      let bundler = DominatorsBundler::default();
      let params = BundlerParams {
        asset_graph: &asset_graph,
        entry_points: &[NodeIndex::new(asset1), NodeIndex::new(asset2)],
      };
      let result = bundler.bundle(params).unwrap();
      println!("{}", result.print_dot(&asset_graph));
      let mut bundles = result.into_bundles();
      assert_eq!(bundles.len(), 4);
      bundles.sort_by(|bundle1, bundle2| bundle1.assets[0].cmp(&bundle2.assets[0]));
      assert_eq!(bundles[0].assets, vec![asset1]);
      assert_eq!(bundles[1].assets, vec![asset2, asset3]);
      assert_eq!(bundles[2].assets, vec![asset4, asset5]);
      assert_eq!(bundles[3].assets, vec![asset6]);
    }
  }
}
