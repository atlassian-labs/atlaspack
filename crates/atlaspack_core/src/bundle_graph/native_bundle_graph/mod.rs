pub mod types;

pub use types::{NativeBundleGraphEdgeType, NativeBundleGraphNode, NodeId};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

use crate::asset_graph::{AssetGraph, AssetGraphNode};
use crate::bundle_graph::BundleGraph;
use crate::types::{Asset, Bundle, Dependency, Target};

/// PetGraph-backed bundle graph, modelled similarly to `AssetGraph`.
#[derive(Clone, Debug)]
pub struct NativeBundleGraph {
  pub graph: StableDiGraph<NodeId, NativeBundleGraphEdgeType>,
  nodes: Vec<NativeBundleGraphNode>,
  node_id_to_node_index: HashMap<NodeId, NodeIndex>,
  content_key_to_node_id: HashMap<String, NodeId>,
  root_node_id: NodeId,

  pub public_id_by_asset_id: HashMap<String, String>,
  pub asset_public_ids: HashSet<String>,
  pub bundle_public_ids: HashSet<String>,
}

impl Default for NativeBundleGraph {
  fn default() -> Self {
    Self::new()
  }
}

impl PartialEq for NativeBundleGraph {
  fn eq(&self, other: &Self) -> bool {
    if self.nodes != other.nodes {
      return false;
    }
    if self.content_key_to_node_id != other.content_key_to_node_id {
      return false;
    }
    if self.public_id_by_asset_id != other.public_id_by_asset_id {
      return false;
    }
    if self.asset_public_ids != other.asset_public_ids {
      return false;
    }

    let mut self_edges: Vec<(NodeId, NodeId, NativeBundleGraphEdgeType)> = self
      .graph
      .edge_references()
      .filter_map(|e| {
        let from = *self.graph.node_weight(e.source())?;
        let to = *self.graph.node_weight(e.target())?;
        Some((from, to, *e.weight()))
      })
      .collect();
    let mut other_edges: Vec<(NodeId, NodeId, NativeBundleGraphEdgeType)> = other
      .graph
      .edge_references()
      .filter_map(|e| {
        let from = *other.graph.node_weight(e.source())?;
        let to = *other.graph.node_weight(e.target())?;
        Some((from, to, *e.weight()))
      })
      .collect();

    self_edges.sort_by_key(|(f, t, w)| (*f, *t, *w as u8));
    other_edges.sort_by_key(|(f, t, w)| (*f, *t, *w as u8));

    self_edges == other_edges
  }
}

impl Eq for NativeBundleGraph {}

impl NativeBundleGraph {
  pub fn new() -> Self {
    let mut graph = StableDiGraph::new();
    let mut node_id_to_node_index = HashMap::new();
    let nodes = vec![NativeBundleGraphNode::Root];
    let root_node_id = 0;
    node_id_to_node_index.insert(root_node_id, graph.add_node(root_node_id));

    Self {
      graph,
      nodes,
      node_id_to_node_index,
      content_key_to_node_id: HashMap::new(),
      root_node_id,
      public_id_by_asset_id: HashMap::new(),
      asset_public_ids: HashSet::new(),
      bundle_public_ids: HashSet::new(),
    }
  }

  pub fn from_asset_graph(asset_graph: &AssetGraph) -> Self {
    let mut bundle_graph = NativeBundleGraph::new();

    for node in asset_graph.nodes() {
      match node {
        AssetGraphNode::Root => {
          bundle_graph
            .content_key_to_node_id
            .insert("@@root".into(), 0);
        }
        AssetGraphNode::Entry => {}
        AssetGraphNode::Asset(asset) => {
          bundle_graph.add_asset(asset.clone(), true);
        }
        AssetGraphNode::Dependency(dep) => {
          bundle_graph.add_dependency(dep.clone(), true);
        }
      }
    }

    // Copy edges as Null edges
    for edge in asset_graph.graph.edge_references() {
      let nodes = asset_graph.graph.node_weights().collect::<Vec<_>>();
      let from_id = *nodes[edge.source().index()];
      let to_id = *nodes[edge.target().index()];

      bundle_graph.add_edge(&from_id, &to_id, NativeBundleGraphEdgeType::Null);
    }

    // Assign public ids for assets
    for node in bundle_graph.nodes.iter() {
      if let NativeBundleGraphNode::Asset(asset) = node {
        let public_id = generate_public_id(&asset.id, |candidate| {
          bundle_graph.asset_public_ids.contains(candidate)
        });
        bundle_graph.asset_public_ids.insert(public_id.clone());
        bundle_graph
          .public_id_by_asset_id
          .insert(asset.id.clone(), public_id);
      }
    }

    bundle_graph
  }

  pub fn nodes(&self) -> impl Iterator<Item = &NativeBundleGraphNode> {
    self.nodes.iter()
  }

  pub fn root_node(&self) -> NodeId {
    self.root_node_id
  }

  pub fn get_node(&self, idx: &NodeId) -> Option<&NativeBundleGraphNode> {
    self.nodes.get(*idx)
  }

  pub fn get_node_id_by_content_key(&self, content_key: &str) -> Option<&NodeId> {
    self.content_key_to_node_id.get(content_key)
  }

  pub fn get_outgoing_neighbors(&self, node_id: &NodeId) -> Vec<NodeId> {
    self
      .graph
      .neighbors_directed(self.node_id_to_node_index[node_id], Direction::Outgoing)
      .filter_map(|node_index| self.graph.node_weight(node_index).copied())
      .collect()
  }

  fn add_node(&mut self, content_key: String, node: NativeBundleGraphNode, cached: bool) -> NodeId {
    let node_id = if let Some(existing_node_id) = self.content_key_to_node_id.get(&content_key) {
      if !cached {
        self.nodes[*existing_node_id] = node;
      }
      *existing_node_id
    } else {
      let node_id = self.nodes.len();
      self.nodes.push(node);
      self.content_key_to_node_id.insert(content_key, node_id);
      node_id
    };

    let node_index = self.graph.add_node(node_id);
    self.node_id_to_node_index.insert(node_id, node_index);
    node_id
  }

  pub fn add_asset(&mut self, asset: Arc<Asset>, cached: bool) -> NodeId {
    self.add_node(
      asset.id.clone(),
      NativeBundleGraphNode::Asset(asset),
      cached,
    )
  }

  pub fn add_dependency(&mut self, dependency: Arc<Dependency>, cached: bool) -> NodeId {
    self.add_node(
      dependency.id(),
      NativeBundleGraphNode::Dependency(dependency),
      cached,
    )
  }

  pub fn add_edge(
    &mut self,
    from_id: &NodeId,
    to_id: &NodeId,
    edge_type: NativeBundleGraphEdgeType,
  ) {
    self.graph.add_edge(
      self.node_id_to_node_index[from_id],
      self.node_id_to_node_index[to_id],
      edge_type,
    );
  }

  pub fn add_bundle_group(&mut self, id: String, target: Target, entry_asset_id: String) -> NodeId {
    self.add_node(
      id,
      NativeBundleGraphNode::BundleGroup {
        target,
        entry_asset_id,
      },
      false,
    )
  }

  pub fn add_bundle(&mut self, mut bundle: Bundle) -> NodeId {
    if bundle.public_id.is_none() {
      let public_id = generate_public_id(&bundle.id, |candidate| {
        self.bundle_public_ids.contains(candidate)
      });
      self.bundle_public_ids.insert(public_id.clone());
      bundle.public_id = Some(public_id);
    }

    self.add_node(
      bundle.id.clone(),
      NativeBundleGraphNode::Bundle(bundle),
      false,
    )
  }
}

const BASE62_ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn base62_encode(bytes: &[u8]) -> String {
  if bytes.is_empty() {
    return String::new();
  }

  let mut num = bytes.iter().fold(0u128, |acc, &b| acc * 256 + b as u128);
  if num == 0 {
    return "0".to_string();
  }

  let mut result = Vec::new();
  while num > 0 {
    let remainder = (num % 62) as usize;
    result.push(BASE62_ALPHABET[remainder]);
    num /= 62;
  }

  result.reverse();
  String::from_utf8(result).unwrap_or_default()
}

pub fn generate_public_id<F>(id: &str, already_exists: F) -> String
where
  F: Fn(&str) -> bool,
{
  let mut bytes = Vec::with_capacity(id.len() / 2);
  let mut i = 0;
  while i + 1 < id.len() {
    if let Ok(b) = u8::from_str_radix(&id[i..i + 2], 16) {
      bytes.push(b);
    }
    i += 2;
  }

  let encoded = base62_encode(&bytes);

  for end in 5..=encoded.len() {
    let candidate = &encoded[..end];
    if !already_exists(candidate) {
      return candidate.to_string();
    }
  }

  panic!("Original id was not unique: {}", id);
}

impl BundleGraph for NativeBundleGraph {
  fn get_bundles(&self) -> Vec<&Bundle> {
    self
      .nodes
      .iter()
      .filter_map(|n| match n {
        NativeBundleGraphNode::Bundle(b) => Some(b),
        _ => None,
      })
      .collect()
  }

  fn get_bundle_assets(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
    let bundle_node_id = self
      .get_node_id_by_content_key(&bundle.id)
      .ok_or_else(|| anyhow::anyhow!("Bundle {} not found in bundle graph", bundle.id))?;

    let bundle_node_index = self
      .node_id_to_node_index
      .get(bundle_node_id)
      .ok_or_else(|| anyhow::anyhow!("Bundle node index missing for {}", bundle.id))?;

    let assets = self
      .graph
      .edges_directed(*bundle_node_index, Direction::Outgoing)
      .filter_map(|e| {
        if *e.weight() != NativeBundleGraphEdgeType::Contains {
          return None;
        }
        let to_id = *self.graph.node_weight(e.target())?;
        match self.nodes.get(to_id)? {
          NativeBundleGraphNode::Asset(a) => Some(a.as_ref()),
          _ => None,
        }
      })
      .collect();

    Ok(assets)
  }

  fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle> {
    let node_id = self.get_node_id_by_content_key(id)?;
    match self.nodes.get(*node_id)? {
      NativeBundleGraphNode::Bundle(b) => Some(b),
      _ => None,
    }
  }

  fn get_public_asset_id(&self, asset_id: &str) -> Option<&str> {
    self.public_id_by_asset_id.get(asset_id).map(|s| s.as_str())
  }

  fn get_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
    let asset_node_id = self
      .get_node_id_by_content_key(&asset.id)
      .ok_or_else(|| anyhow::anyhow!("Asset {} not found in bundle graph", asset.id))?;

    let asset_node_index = self
      .node_id_to_node_index
      .get(asset_node_id)
      .ok_or_else(|| anyhow::anyhow!("Asset node index missing for {}", asset.id))?;

    let deps = self
      .graph
      .edges_directed(*asset_node_index, Direction::Outgoing)
      .filter_map(|e| {
        // In the base graph copied from AssetGraph, asset -> dependency edges are Null.
        if *e.weight() != NativeBundleGraphEdgeType::Null {
          return None;
        }
        let to_id = *self.graph.node_weight(e.target())?;
        match self.nodes.get(to_id)? {
          NativeBundleGraphNode::Dependency(d) => Some(d.as_ref()),
          _ => None,
        }
      })
      .collect();

    Ok(deps)
  }

  fn get_resolved_asset(
    &self,
    dependency: &Dependency,
    _bundle: &Bundle,
  ) -> anyhow::Result<Option<&Asset>> {
    let dep_node_id = match self.get_node_id_by_content_key(&dependency.id()) {
      Some(id) => id,
      None => return Ok(None),
    };

    let dep_node_index = self
      .node_id_to_node_index
      .get(dep_node_id)
      .ok_or_else(|| anyhow::anyhow!("Dependency node index missing for {}", dependency.id()))?;

    let resolved = self
      .graph
      .edges_directed(*dep_node_index, Direction::Outgoing)
      .filter_map(|e| {
        if *e.weight() != NativeBundleGraphEdgeType::Null {
          return None;
        }
        let to_id = *self.graph.node_weight(e.target())?;
        match self.nodes.get(to_id)? {
          NativeBundleGraphNode::Asset(a) => Some(a.as_ref()),
          _ => None,
        }
      })
      .next();

    Ok(resolved)
  }

  fn is_dependency_skipped(&self, _dependency: &Dependency) -> bool {
    false
  }
}
