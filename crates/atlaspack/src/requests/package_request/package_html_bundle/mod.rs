use std::{
  collections::{HashMap, HashSet},
  io::Write,
  sync::Arc,
};

use atlaspack_core::bundle_graph::{
  BundleGraph, BundleGraphBundle, BundleGraphEdge, BundleGraphNode, BundleRef,
};
use atlaspack_plugin_transformer_html::dom_visitor::{walk, DomTraversalOperation, DomVisitor};
use html5ever::namespace_url;
use markup5ever::{expanded_name, local_name, ns, QualName};
use markup5ever_rcdom::{Handle, NodeData};
use petgraph::{graph::NodeIndex, visit::EdgeRef};

use crate::requests::package_request::{
  package_bundle,
  package_html_bundle::find_html_reference_nodes::{
    rewrite_html_reference, RewriteHTMLReferenceParams,
  },
  AssetDataProvider, PackageBundleParams,
};

mod find_html_reference_nodes;
mod rewrite_html_dependencies_visitor;

pub fn get_all_referenced_bundles(
  bundle_graph: &BundleGraph,
  bundle_node_index: NodeIndex,
) -> Vec<(Arc<BundleGraphBundle>, NodeIndex)> {
  let mut result = Vec::new();
  let mut visited_bundles = HashSet::new();

  let mut bundle_stack = vec![bundle_node_index];
  while let Some(bundle_node_index) = bundle_stack.pop() {
    if visited_bundles.contains(&bundle_node_index) {
      continue;
    }
    visited_bundles.insert(bundle_node_index);

    let referenced_bundles = bundle_graph
      .graph()
      .edges_directed(bundle_node_index, petgraph::Direction::Outgoing)
      .map(|e| (e.target(), e.weight()))
      .collect::<Vec<_>>();

    for (referenced_bundle_node_index, edge_weight) in referenced_bundles.into_iter() {
      let dependency_id = match edge_weight {
        BundleGraphEdge::BundleSyncLoads(dependency_node) => Some(dependency_node.id()),
        BundleGraphEdge::BundleAsyncLoads(dependency_node) => Some(dependency_node.id()),
        _ => None,
      };

      if let Some(dependency_id) = dependency_id {
        let target_bundle_node = bundle_graph
          .graph()
          .node_weight(referenced_bundle_node_index)
          .unwrap();

        let target_bundle = target_bundle_node.as_bundle().unwrap();
        let target_bundle_path = target_bundle
          .bundle
          .name
          .as_ref()
          .unwrap()
          .as_str()
          .to_string();

        result.push((target_bundle.clone(), referenced_bundle_node_index));
      }

      // Continue traversal regardless, so we visit all reachable bundles.
      bundle_stack.push(referenced_bundle_node_index);
    }
  }

  result
}

fn get_referenced_paths_by_dependency_id(
  bundle_graph: &BundleGraph,
  bundle_node_index: NodeIndex,
) -> HashMap<String, Vec<String>> {
  let mut referenced_paths_by_dependency_id: HashMap<String, Vec<String>> = HashMap::new();
  let mut visited_bundles = HashSet::new();

  let mut bundle_stack = vec![bundle_node_index];
  while let Some(bundle_node_index) = bundle_stack.pop() {
    if visited_bundles.contains(&bundle_node_index) {
      continue;
    }
    visited_bundles.insert(bundle_node_index);

    let referenced_bundles = bundle_graph
      .graph()
      .edges_directed(bundle_node_index, petgraph::Direction::Outgoing)
      .map(|e| (e.target(), e.weight()))
      .collect::<Vec<_>>();

    for (referenced_bundle_node_index, edge_weight) in referenced_bundles.into_iter() {
      let dependency_id = match edge_weight {
        BundleGraphEdge::BundleSyncLoads(dependency_node) => Some(dependency_node.id()),
        BundleGraphEdge::BundleAsyncLoads(dependency_node) => Some(dependency_node.id()),
        _ => None,
      };

      if let Some(dependency_id) = dependency_id {
        let target_bundle_node = bundle_graph
          .graph()
          .node_weight(referenced_bundle_node_index)
          .unwrap();

        let target_bundle = target_bundle_node.as_bundle().unwrap();
        let target_bundle_path = target_bundle
          .bundle
          .name
          .as_ref()
          .unwrap()
          .as_str()
          .to_string();

        referenced_paths_by_dependency_id
          .entry(dependency_id)
          .or_default()
          .push(target_bundle_path);
      }

      // Continue traversal regardless, so we visit all reachable bundles.
      bundle_stack.push(referenced_bundle_node_index);
    }
  }

  referenced_paths_by_dependency_id
}

pub fn package_html_bundle<ADP: AssetDataProvider>(
  PackageBundleParams {
    bundle,
    bundle_node_index,
    asset_data_provider,
    bundle_graph,
    options,
    project_root,
    packager,
  }: PackageBundleParams<'_, ADP>,
  writer: &mut impl Write,
) -> anyhow::Result<()> {
  assert!(bundle.assets.node_count() == 1);
  let bundle_asset_ref = bundle.assets.node_weights().next().unwrap();
  let bundle_asset = asset_data_provider.get_asset_code(bundle_asset_ref.id())?;

  let prelude = include_str!("../../packager_runtime/prelude.js");
  writer.write_all(format!("<script>{}</script>", prelude).as_bytes())?;

  let referenced_paths_by_dependency_id =
    get_referenced_paths_by_dependency_id(bundle_graph, bundle_node_index);

  let dom = atlaspack_plugin_transformer_html::parse_html(&bundle_asset)?;
  let mut find_reference_nodes = find_html_reference_nodes::FindHTMLReferenceNodesVisitor::new();
  walk(dom.document.clone(), &mut find_reference_nodes);
  let references = find_reference_nodes.into_references();

  let referenced_bundles = bundle_graph
    .referenced_bundles_for_index(bundle_node_index)
    .into_iter()
    .map(|reference| (reference.dependency().specifier().to_string(), reference))
    .collect::<HashMap<_, _>>();

  let mut contents_by_dependency_specifier = HashMap::new();
  for reference in &references {
    // println!("reference: {:?}", reference);
    // println!("referenced_bundles: {:#?}", referenced_bundles);
    if reference.reference_type() == &find_html_reference_nodes::HTMLReferenceType::InlineScript {
      let dependency_specifier = reference.dependency_specifier();

      if let Some(reference) = referenced_bundles.get(dependency_specifier) {
        let mut inline_writer = Vec::new();

        // TODO: This is a mess
        let referenced_bundles =
          get_all_referenced_bundles(bundle_graph, reference.bundle_node_index());

        println!(
          "inline - referenced_bundles: {:#?}",
          referenced_bundles.len()
        );

        for (referenced_bundle, referenced_bundle_node_index) in referenced_bundles {
          let _ = package_bundle(
            PackageBundleParams {
              bundle: &*referenced_bundle,
              bundle_node_index: referenced_bundle_node_index,
              options: options.clone(),
              project_root: project_root.clone(),
              asset_data_provider,
              bundle_graph,
              packager,
            },
            &mut inline_writer,
          )?;
        }

        let _ = package_bundle(
          PackageBundleParams {
            bundle: reference.bundle_graph_bundle(),
            bundle_node_index: reference.bundle_node_index(),
            options: options.clone(),
            project_root: project_root.clone(),
            asset_data_provider,
            bundle_graph,
            packager,
          },
          &mut inline_writer,
        )?;

        contents_by_dependency_specifier.insert(
          dependency_specifier.to_string(),
          String::from_utf8(inline_writer)?,
        );
      }
    }
  }

  for reference in &references {
    rewrite_html_reference(
      RewriteHTMLReferenceParams {
        referenced_paths_by_dependency_id: &referenced_paths_by_dependency_id,
        contents_by_dependency_specifier: &contents_by_dependency_specifier,
      },
      reference,
    );
  }

  let html = String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom)?).unwrap();
  writer.write_all(html.as_bytes())?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::{collections::HashSet, sync::Arc};

  use atlaspack_core::{
    asset_graph::{AssetGraph, AssetNode, DependencyNode, DependencyState},
    bundle_graph::{AssetRef, BundleDependency, BundleGraphBundle},
    types::{
      Asset, AssetId, Bundle, BundleBehavior, Code, Dependency, Environment, FileType, Target,
    },
  };
  use petgraph::prelude::StableDiGraph;

  use crate::requests::package_request::InMemoryAssetDataProvider;

  use super::*;

  #[test]
  fn test_get_referenced_paths_by_dependency_id_traverses_graph() {
    // Build bundle graph: root(html) -> jsA(dep:"a"), root(html) -> jsB(dep:"b"), jsB -> jsC(dep:"c")
    let mut bundle_graph = BundleGraph::new();

    // HTML root bundle
    let html_bundle = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: FileType::Html,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: String::new(),
        id: "root".into(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: Some("index.html".into()),
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: StableDiGraph::default(),
    };
    let _root_idx = bundle_graph.add_bundle(BundleGraphEdge::RootEntryOf, html_bundle);

    // jsA bundle
    let js_a = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: FileType::Js,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: String::new(),
        id: "a".into(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: Some("a.js".into()),
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: StableDiGraph::default(),
    };
    let dep_a = BundleDependency::new(
      &DependencyNode {
        dependency: Arc::new(Dependency {
          ..Default::default()
        }),
        requested_symbols: HashSet::new(),
        state: DependencyState::Resolved,
      },
      StableDiGraph::<AssetRef, ()>::default().add_node(AssetRef::new(
        AssetNode {
          asset: Asset {
            id: AssetId::default(),
            ..Asset::default()
          },
          requested_symbols: Default::default(),
        },
        NodeIndex::new(0),
      )),
    );
    let _a_idx = bundle_graph.add_bundle(BundleGraphEdge::BundleSyncLoads(dep_a.clone()), js_a);

    // jsB bundle
    let js_b = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: FileType::Js,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: String::new(),
        id: "b".into(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: Some("b.js".into()),
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: StableDiGraph::default(),
    };
    let dep_b = BundleDependency::new(
      &DependencyNode {
        dependency: Arc::new(Dependency {
          ..Default::default()
        }),
        requested_symbols: HashSet::new(),
        state: DependencyState::Resolved,
      },
      StableDiGraph::<AssetRef, ()>::default().add_node(AssetRef::new(
        AssetNode {
          asset: Asset {
            id: AssetId::default(),
            ..Asset::default()
          },
          requested_symbols: Default::default(),
        },
        NodeIndex::new(0),
      )),
    );
    let _b_idx = bundle_graph.add_bundle(BundleGraphEdge::BundleAsyncLoads(dep_b.clone()), js_b);

    // jsC bundle
    let js_c = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: FileType::Js,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: String::new(),
        id: "c".into(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: Some("c.js".into()),
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: StableDiGraph::default(),
    };
    let dep_c = BundleDependency::new(
      &DependencyNode {
        dependency: Arc::new(Dependency {
          ..Default::default()
        }),
        requested_symbols: HashSet::new(),
        state: DependencyState::Resolved,
      },
      StableDiGraph::<AssetRef, ()>::default().add_node(AssetRef::new(
        AssetNode {
          asset: Asset {
            id: AssetId::default(),
            ..Asset::default()
          },
          requested_symbols: Default::default(),
        },
        NodeIndex::new(0),
      )),
    );
    let _c_idx = bundle_graph.add_bundle(BundleGraphEdge::BundleAsyncLoads(dep_c.clone()), js_c);

    // Connect root->A and root->B, and B->C via add_bundle already implying edges by API
    // The add_bundle calls above added edges from root to a_idx and b_idx, and from b_idx to c

    let map = get_referenced_paths_by_dependency_id(&bundle_graph, bundle_graph.root());
    assert!(map.get(&dep_a.id()).unwrap().contains(&"a.js".to_string()));
    assert!(map.get(&dep_b.id()).unwrap().contains(&"b.js".to_string()));
  }

  #[test]
  fn test_package_html_bundle() {
    let _bundle_asset = r#"
    <html>
      <body>
        <script src="dep"></script>
      </body>
    </html>
    "#;

    let mut asset_graph = AssetGraph::new();
    let mut assets = StableDiGraph::default();
    let asset_graph_node_index = asset_graph.add_asset(Asset {
      code: Code::new(
        r#"
<html>
  <body>
    <script src="dep"></script>
  </body>
</html>
      "#
        .as_bytes()
        .to_vec(),
      ),
      ..Asset::default()
    });
    let asset_node = asset_graph.get_asset_node(&asset_graph_node_index).unwrap();

    let _ = assets.add_node(AssetRef::new(asset_node.clone(), asset_graph_node_index));
    let bundle = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: FileType::Js,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: "hash".to_string(),
        id: "bundle".to_string(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: Some("bundle".to_string()),
        needs_stable_name: true,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets,
    };
    let mut bundle_graph = BundleGraph::new();
    let bundle_node_index = bundle_graph.add_bundle(BundleGraphEdge::RootEntryOf, bundle);

    let mut assets = StableDiGraph::default();
    let script_asset = assets.add_node(AssetRef::new(asset_node.clone(), asset_graph_node_index));
    let script_bundle = BundleGraphBundle {
      bundle: Bundle {
        bundle_behavior: Some(BundleBehavior::Isolated),
        bundle_type: FileType::Js,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: "hash".to_string(),
        id: "bundle".to_string(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: Some("bundle".to_string()),
        needs_stable_name: true,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets,
    };
    let _ = bundle_graph.add_bundle(
      BundleGraphEdge::BundleAsyncLoads(BundleDependency::new(
        &DependencyNode {
          dependency: Arc::new(Dependency {
            ..Default::default()
          }),
          requested_symbols: HashSet::new(),
          state: DependencyState::Resolved,
        },
        script_asset,
      )),
      script_bundle,
    );

    let bundle = bundle_graph
      .graph()
      .node_weight(bundle_node_index)
      .unwrap()
      .as_bundle()
      .unwrap();

    let asset_graph = Arc::new(asset_graph);
    let asset_data_provider = InMemoryAssetDataProvider::new(asset_graph);

    let mut writer = Vec::new();
    let _dom = package_html_bundle(
      PackageBundleParams {
        bundle: &bundle,
        bundle_node_index,
        asset_data_provider: &asset_data_provider,
        bundle_graph: &bundle_graph,
        options: todo!(),
        project_root: todo!(),
        packager: todo!(),
      },
      &mut writer,
    );
  }
}
