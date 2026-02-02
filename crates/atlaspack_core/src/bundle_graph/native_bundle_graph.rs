use std::collections::{HashMap, HashSet};

use crate::asset_graph::{AssetGraph, AssetGraphNode};

/// Minimal native BundleGraph representation used during the native bundling migration.
///
/// Note: This is intentionally lightweight and currently only supports a subset of the JS
/// bundle graph functionality needed by the request/serialization layers.
#[derive(Debug, Default, Clone)]
pub struct NativeBundleGraph {
  pub node_count: usize,
  pub edges: Vec<(u32, u32)>,

  /// Maps full asset IDs to concise public IDs.
  pub public_id_by_asset_id: HashMap<String, String>,

  /// Set of all assigned asset public IDs.
  pub asset_public_ids: HashSet<String>,
}

impl NativeBundleGraph {
  pub fn from_asset_graph(asset_graph: &AssetGraph) -> Self {
    // Node ids in `AssetGraph` are stable numeric indices.
    let node_count = asset_graph.nodes().count();

    // AssetGraph exposes edges as a flat list [from, to, from, to, ...]
    let flat_edges = asset_graph.edges();
    let mut edges: Vec<(u32, u32)> = Vec::with_capacity(flat_edges.len() / 2);
    for pair in flat_edges.chunks_exact(2) {
      edges.push((pair[0], pair[1]));
    }

    let mut public_id_by_asset_id: HashMap<String, String> = HashMap::new();
    let mut asset_public_ids: HashSet<String> = HashSet::new();

    // Assign stable public IDs to all asset nodes.
    for node in asset_graph.nodes() {
      if let AssetGraphNode::Asset(asset) = node {
        let public_id = generate_public_id(&asset.id, |candidate| {
          asset_public_ids.contains(candidate)
        });
        asset_public_ids.insert(public_id.clone());
        public_id_by_asset_id.insert(asset.id.clone(), public_id);
      }
    }

    NativeBundleGraph {
      node_count,
      edges,
      public_id_by_asset_id,
      asset_public_ids,
    }
  }
}

const BASE62_ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn base62_encode(bytes: &[u8]) -> String {
  if bytes.is_empty() {
    return String::new();
  }

  // NOTE: Simplified big-int approach.
  // Asset ids are expected to fit within 16 bytes (u128) in practice.
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

/// Generate a public ID from a hex asset id.
///
/// Mirrors `packages/core/core/src/utils.ts#getPublicId`:
/// - base62 encode the bytes
/// - shortest unique prefix, minimum length 5
pub fn generate_public_id<F>(id: &str, already_exists: F) -> String
where
  F: Fn(&str) -> bool,
{
  // Hex decode
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn public_id_is_min_5_chars_and_unique() {
    let mut existing = HashSet::new();
    let id1 = generate_public_id("deadbeef12345678", |s| existing.contains(s));
    assert!(id1.len() >= 5);
    existing.insert(id1.clone());

    let id2 = generate_public_id("cafebabe87654321", |s| existing.contains(s));
    assert!(id2.len() >= 5);
    assert_ne!(id1, id2);
  }
}

