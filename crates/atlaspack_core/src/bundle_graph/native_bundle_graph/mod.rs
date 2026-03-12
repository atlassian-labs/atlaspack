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
use crate::types::{Asset, Bundle, BundleBehavior, Dependency, Target};

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
    let nodes = asset_graph.graph.node_weights().collect::<Vec<_>>();
    for edge in asset_graph.graph.edge_references() {
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

  pub fn get_neighbors_by_edge_type(
    &self,
    node_id: &NodeId,
    edge_type: NativeBundleGraphEdgeType,
  ) -> Vec<NodeId> {
    let Some(node_index) = self.node_id_to_node_index.get(node_id) else {
      return vec![];
    };

    self
      .graph
      .edges_directed(*node_index, Direction::Outgoing)
      .filter_map(|e| {
        if *e.weight() != edge_type {
          return None;
        }

        self.graph.node_weight(e.target()).copied()
      })
      .collect()
  }

  pub fn has_edge(
    &self,
    from_id: &NodeId,
    to_id: &NodeId,
    edge_type: NativeBundleGraphEdgeType,
  ) -> bool {
    let Some(&from_index) = self.node_id_to_node_index.get(from_id) else {
      return false;
    };
    let Some(&to_index) = self.node_id_to_node_index.get(to_id) else {
      return false;
    };

    self
      .graph
      .edges_connecting(from_index, to_index)
      .any(|e| *e.weight() == edge_type)
  }

  pub fn get_incoming_neighbors_by_edge_type(
    &self,
    node_id: &NodeId,
    edge_type: NativeBundleGraphEdgeType,
  ) -> Vec<NodeId> {
    let Some(node_index) = self.node_id_to_node_index.get(node_id) else {
      return vec![];
    };

    self
      .graph
      .edges_directed(*node_index, Direction::Incoming)
      .filter_map(|e| {
        if *e.weight() != edge_type {
          return None;
        }

        self.graph.node_weight(e.source()).copied()
      })
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

  pub fn remove_edge(
    &mut self,
    from_id: &NodeId,
    to_id: &NodeId,
    edge_type: NativeBundleGraphEdgeType,
  ) {
    use petgraph::stable_graph::EdgeIndex;

    let from_index = self.node_id_to_node_index[from_id];
    let to_index = self.node_id_to_node_index[to_id];

    let mut to_remove: Vec<EdgeIndex> = Vec::new();
    for edge in self.graph.edges_connecting(from_index, to_index) {
      if *edge.weight() == edge_type {
        to_remove.push(edge.id());
      }
    }

    for edge_id in to_remove {
      self.graph.remove_edge(edge_id);
    }
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

  // THIS IS DEFINITELY NOT PRODUCTION READY CODE!
  #[tracing::instrument(level = "info", skip_all)]
  pub fn name_bundles(&mut self) {
    // Two passes: we need to read asset nodes while building names, then mutate bundle nodes.
    let names: HashMap<String, String> = self
      .nodes
      .iter()
      .filter_map(|node| {
        let NativeBundleGraphNode::Bundle(b) = node else {
          return None;
        };
        let stem = b
          .entry_asset_ids
          .first()
          .and_then(|id| self.get_node_id_by_content_key(id))
          .and_then(|nid| self.nodes.get(*nid))
          .and_then(|n| match n {
            NativeBundleGraphNode::Asset(asset) => asset
              .file_path
              .file_stem()
              .and_then(|s| s.to_str())
              .map(String::from),
            _ => None,
          })
          .unwrap_or_else(|| "bundle".to_string());
        let ext = b.bundle_type.extension();
        let name = if b.needs_stable_name == Some(true) {
          format!("{}.{}", stem, ext)
        } else {
          format!("{}.HASH_REF_{}.{}", stem, b.id, ext)
        };
        Some((b.id.clone(), name))
      })
      .collect();

    for node in self.nodes.iter_mut() {
      if let NativeBundleGraphNode::Bundle(b) = node
        && let Some(name) = names.get(&b.id)
      {
        b.name = Some(name.clone());
      }
    }
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

  fn get_incoming_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>> {
    let asset_node_id = self
      .get_node_id_by_content_key(&asset.id)
      .ok_or_else(|| anyhow::anyhow!("Asset {} not found in bundle graph", asset.id))?;

    let asset_node_index = self
      .node_id_to_node_index
      .get(asset_node_id)
      .ok_or_else(|| anyhow::anyhow!("Asset node index missing for {}", asset.id))?;

    self
      .graph
      .edges_directed(*asset_node_index, Direction::Incoming)
      .filter_map(|e| {
        if *e.weight() != NativeBundleGraphEdgeType::Null {
          return None;
        }
        let from_id = *self.graph.node_weight(e.source())?;
        match self.nodes.get(from_id)? {
          NativeBundleGraphNode::Dependency(d) => Some(Ok(d.as_ref())),
          other => Some(Err(anyhow::anyhow!(
            "Expected dependency node on incoming Null edge, got {:?}",
            other
          ))),
        }
      })
      .collect()
  }

  fn get_bundle_assets_in_source_order(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
    let bundle_node_id = self
      .get_node_id_by_content_key(&bundle.id)
      .ok_or_else(|| anyhow::anyhow!("Bundle {} not found in bundle graph", bundle.id))?;

    let bundle_node_index = self
      .node_id_to_node_index
      .get(bundle_node_id)
      .ok_or_else(|| anyhow::anyhow!("Bundle node index missing for {}", bundle.id))?;

    // Collect asset node indices contained in this bundle.
    let bundle_asset_ids: HashSet<NodeId> = self
      .graph
      .edges_directed(*bundle_node_index, Direction::Outgoing)
      .filter_map(|e| {
        if *e.weight() != NativeBundleGraphEdgeType::Contains {
          return None;
        }
        let to_id = *self.graph.node_weight(e.target())?;
        if matches!(self.nodes.get(to_id)?, NativeBundleGraphNode::Asset(_)) {
          Some(to_id)
        } else {
          None
        }
      })
      .collect();

    // DFS post-order: dependencies before dependents (correct CSS cascade order).
    let mut result: Vec<&Asset> = Vec::with_capacity(bundle_asset_ids.len());
    let mut visited: HashSet<NodeId> = HashSet::new();

    // Entry assets: bundle assets with no incoming Null edge from another bundle asset
    // (asset → dep → asset two-hop).
    let is_root = |asset_id: NodeId| -> bool {
      let Some(&asset_idx) = self.node_id_to_node_index.get(&asset_id) else {
        return true;
      };
      !self
        .graph
        .edges_directed(asset_idx, Direction::Incoming)
        .any(|e| {
          if *e.weight() != NativeBundleGraphEdgeType::Null {
            return false;
          }
          let dep_id = match self.graph.node_weight(e.source()) {
            Some(&id) => id,
            None => return false,
          };
          let Some(&dep_idx) = self.node_id_to_node_index.get(&dep_id) else {
            return false;
          };
          self
            .graph
            .edges_directed(dep_idx, Direction::Incoming)
            .any(|dep_in| {
              if *dep_in.weight() != NativeBundleGraphEdgeType::Null {
                return false;
              }
              match self.graph.node_weight(dep_in.source()) {
                Some(&src_id) => bundle_asset_ids.contains(&src_id),
                None => false,
              }
            })
        })
    };

    // Seed with entry_asset_ids order first for deterministic output matching JS traversal.
    let mut seen_entries: HashSet<NodeId> = HashSet::new();
    let mut entry_ids: Vec<NodeId> = bundle
      .entry_asset_ids
      .iter()
      .filter_map(|id| self.get_node_id_by_content_key(id).copied())
      .filter(|&nid| bundle_asset_ids.contains(&nid) && is_root(nid))
      .inspect(|&nid| {
        seen_entries.insert(nid);
      })
      .collect();

    // Append remaining roots sorted by NodeId for stability.
    let mut remaining_roots: Vec<NodeId> = bundle_asset_ids
      .iter()
      .filter(|&&nid| is_root(nid) && !seen_entries.contains(&nid))
      .copied()
      .collect();
    remaining_roots.sort_unstable();
    entry_ids.extend(remaining_roots);

    // Iterative DFS post-order stack: (node_id, expanded).
    let mut stack: Vec<(NodeId, bool)> = entry_ids.iter().map(|&id| (id, false)).collect();

    while let Some((node_id, expanded)) = stack.pop() {
      if expanded {
        if let Some(NativeBundleGraphNode::Asset(a)) = self.nodes.get(node_id) {
          result.push(a.as_ref());
        }
        continue;
      }

      if visited.contains(&node_id) {
        continue;
      }
      visited.insert(node_id);
      stack.push((node_id, true));

      let Some(&node_idx) = self.node_id_to_node_index.get(&node_id) else {
        continue;
      };

      // Follow asset → dep → asset (Null edges).
      for dep_edge in self.graph.edges_directed(node_idx, Direction::Outgoing) {
        if *dep_edge.weight() != NativeBundleGraphEdgeType::Null {
          continue;
        }
        let dep_id = match self.graph.node_weight(dep_edge.target()) {
          Some(&id) => id,
          None => continue,
        };
        let Some(&dep_idx) = self.node_id_to_node_index.get(&dep_id) else {
          continue;
        };
        for asset_edge in self.graph.edges_directed(dep_idx, Direction::Outgoing) {
          if *asset_edge.weight() != NativeBundleGraphEdgeType::Null {
            continue;
          }
          let child_id = match self.graph.node_weight(asset_edge.target()) {
            Some(&id) => id,
            None => continue,
          };
          if bundle_asset_ids.contains(&child_id) && !visited.contains(&child_id) {
            stack.push((child_id, false));
          }
        }
      }
    }

    Ok(result)
  }

  fn get_referenced_bundle_ids(&self, bundle: &Bundle) -> Vec<String> {
    let Some(bundle_node_id) = self.get_node_id_by_content_key(&bundle.id) else {
      return vec![];
    };
    let Some(bundle_node_index) = self.node_id_to_node_index.get(bundle_node_id) else {
      return vec![];
    };

    self
      .graph
      .edges_directed(*bundle_node_index, Direction::Outgoing)
      .filter_map(|e| {
        if *e.weight() != NativeBundleGraphEdgeType::References {
          return None;
        }
        let to_id = *self.graph.node_weight(e.target())?;
        match self.nodes.get(to_id)? {
          NativeBundleGraphNode::Bundle(b) => Some(b.id.clone()),
          _ => None,
        }
      })
      .collect()
  }

  fn get_inline_bundle_ids(&self, bundle: &Bundle) -> Vec<String> {
    let Some(bundle_node_id) = self.get_node_id_by_content_key(&bundle.id) else {
      return vec![];
    };
    let Some(bundle_node_index) = self.node_id_to_node_index.get(bundle_node_id) else {
      return vec![];
    };

    // Inline bundles appear as neighbours via both Contains (2) and References (4) edges.
    // Collect both to match the JS getInlineBundles() implementation.
    self
      .graph
      .edges_directed(*bundle_node_index, Direction::Outgoing)
      .filter_map(|e| {
        let edge_type = *e.weight();
        if edge_type != NativeBundleGraphEdgeType::Contains
          && edge_type != NativeBundleGraphEdgeType::References
        {
          return None;
        }
        let to_id = *self.graph.node_weight(e.target())?;
        match self.nodes.get(to_id)? {
          NativeBundleGraphNode::Bundle(b)
            if matches!(
              b.bundle_behavior,
              Some(BundleBehavior::Inline) | Some(BundleBehavior::InlineIsolated)
            ) =>
          {
            Some(b.id.clone())
          }
          _ => None,
        }
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{Asset, Bundle, Dependency, Environment, FileType, Target};
  use pretty_assertions::assert_eq;
  use std::sync::Arc;

  fn make_asset(id: &str) -> Arc<Asset> {
    Arc::new(Asset {
      id: id.to_string(),
      ..Asset::default()
    })
  }

  fn make_dependency(id: &str) -> Arc<Dependency> {
    Arc::new(Dependency {
      id: id.to_string(),
      ..Dependency::default()
    })
  }

  fn make_bundle(id: &str, entry_asset_ids: Vec<String>) -> Bundle {
    Bundle {
      id: id.to_string(),
      bundle_type: FileType::Css,
      entry_asset_ids,
      env: Environment::default(),
      hash_reference: String::new(),
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: None,
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      bundle_behavior: None,
      is_placeholder: false,
      target: Target::default(),
    }
  }

  /// `get_incoming_dependencies` must return the single dependency whose Null
  /// edge points to the target asset.
  #[test]
  fn test_get_incoming_dependencies_single_dep() {
    let mut bg = NativeBundleGraph::new();

    let dep = make_dependency("dep1");
    let asset = make_asset("asset1");

    let dep_id = bg.add_dependency(dep.clone(), false);
    let asset_id = bg.add_asset(asset.clone(), false);

    // dep1 --Null--> asset1  (the pattern used in AssetGraph)
    bg.add_edge(&dep_id, &asset_id, NativeBundleGraphEdgeType::Null);

    let incoming = bg.get_incoming_dependencies(&asset).unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].id, "dep1");
  }

  /// `get_bundle_assets_in_source_order` must return assets in DFS post-order
  /// (dependencies before dependents) for a simple 3-asset linear chain:
  ///   asset_a --dep_ab--> asset_b --dep_bc--> asset_c
  /// Expected order: [asset_c, asset_b, asset_a]
  #[test]
  fn test_get_bundle_assets_in_source_order_three_asset_chain() {
    let mut bg = NativeBundleGraph::new();

    let asset_a = make_asset("asset_a");
    let asset_b = make_asset("asset_b");
    let asset_c = make_asset("asset_c");
    let dep_ab = make_dependency("dep_ab");
    let dep_bc = make_dependency("dep_bc");

    let id_a = bg.add_asset(asset_a.clone(), false);
    let id_b = bg.add_asset(asset_b.clone(), false);
    let id_c = bg.add_asset(asset_c.clone(), false);
    let id_dep_ab = bg.add_dependency(dep_ab.clone(), false);
    let id_dep_bc = bg.add_dependency(dep_bc.clone(), false);

    // asset_a -> dep_ab -> asset_b -> dep_bc -> asset_c  (Null edges, mirroring AssetGraph)
    bg.add_edge(&id_a, &id_dep_ab, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_dep_ab, &id_b, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_b, &id_dep_bc, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_dep_bc, &id_c, NativeBundleGraphEdgeType::Null);

    // Bundle contains all three assets. The id must be a hex string for generate_public_id.
    let bundle = make_bundle("aabbccdd11223344", vec!["asset_a".to_string()]);
    let bundle_id = bg.add_bundle(bundle.clone());
    bg.add_edge(&bundle_id, &id_a, NativeBundleGraphEdgeType::Contains);
    bg.add_edge(&bundle_id, &id_b, NativeBundleGraphEdgeType::Contains);
    bg.add_edge(&bundle_id, &id_c, NativeBundleGraphEdgeType::Contains);

    let ordered = bg.get_bundle_assets_in_source_order(&bundle).unwrap();
    let ids: Vec<&str> = ordered.iter().map(|a| a.id.as_str()).collect();

    // Post-order: leaf first, root last.
    assert_eq!(ids, vec!["asset_c", "asset_b", "asset_a"]);
  }

  /// Diamond: A imports B and C, both B and C import D.
  /// D must appear exactly once. B and C must both appear before A. D before B and C.
  #[test]
  fn test_get_bundle_assets_in_source_order_handles_diamond() {
    let mut bg = NativeBundleGraph::new();

    let asset_a = make_asset("asset_a");
    let asset_b = make_asset("asset_b");
    let asset_c = make_asset("asset_c");
    let asset_d = make_asset("asset_d");
    let dep_ab = make_dependency("dep_ab");
    let dep_ac = make_dependency("dep_ac");
    let dep_bd = make_dependency("dep_bd");
    let dep_cd = make_dependency("dep_cd");

    let id_a = bg.add_asset(asset_a.clone(), false);
    let id_b = bg.add_asset(asset_b.clone(), false);
    let id_c = bg.add_asset(asset_c.clone(), false);
    let id_d = bg.add_asset(asset_d.clone(), false);
    let id_dep_ab = bg.add_dependency(dep_ab.clone(), false);
    let id_dep_ac = bg.add_dependency(dep_ac.clone(), false);
    let id_dep_bd = bg.add_dependency(dep_bd.clone(), false);
    let id_dep_cd = bg.add_dependency(dep_cd.clone(), false);

    bg.add_edge(&id_a, &id_dep_ab, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_dep_ab, &id_b, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_a, &id_dep_ac, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_dep_ac, &id_c, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_b, &id_dep_bd, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_dep_bd, &id_d, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_c, &id_dep_cd, NativeBundleGraphEdgeType::Null);
    bg.add_edge(&id_dep_cd, &id_d, NativeBundleGraphEdgeType::Null);

    let bundle = make_bundle("aabbccdd11223344", vec!["asset_a".to_string()]);
    let bundle_id = bg.add_bundle(bundle.clone());
    bg.add_edge(&bundle_id, &id_a, NativeBundleGraphEdgeType::Contains);
    bg.add_edge(&bundle_id, &id_b, NativeBundleGraphEdgeType::Contains);
    bg.add_edge(&bundle_id, &id_c, NativeBundleGraphEdgeType::Contains);
    bg.add_edge(&bundle_id, &id_d, NativeBundleGraphEdgeType::Contains);

    let ordered = bg.get_bundle_assets_in_source_order(&bundle).unwrap();
    let ids: Vec<&str> = ordered.iter().map(|a| a.id.as_str()).collect();

    // D must appear exactly once.
    assert_eq!(
      ids.iter().filter(|&&id| id == "asset_d").count(),
      1,
      "diamond-shared asset must appear exactly once, got: {ids:?}"
    );

    let pos = |target: &str| ids.iter().position(|&id| id == target).unwrap();
    let pos_a = pos("asset_a");
    let pos_b = pos("asset_b");
    let pos_c = pos("asset_c");
    let pos_d = pos("asset_d");

    assert!(pos_d < pos_b, "D must come before B, got: {ids:?}");
    assert!(pos_d < pos_c, "D must come before C, got: {ids:?}");
    assert!(pos_b < pos_a, "B must come before A, got: {ids:?}");
    assert!(pos_c < pos_a, "C must come before A, got: {ids:?}");
  }
}
