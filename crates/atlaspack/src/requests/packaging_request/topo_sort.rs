//! Topological ordering for the bundle reference graph.
//!
//! Bundles must be packaged in dependency order: if bundle A's content embeds bundle B's
//! `hash_reference` placeholder, B must be fully packaged (and its content hash known) before A
//! is packaged. This module provides the functions that derive that ordering via Kahn's BFS
//! algorithm and diagnose cycles with a DFS path-finder.

use std::collections::{HashMap, VecDeque};

use anyhow::anyhow;
use atlaspack_core::bundle_graph::BundleGraph;
use atlaspack_core::types::Bundle;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Mirrors the JS `nameHashForFilename`: returns the last 8 characters of `id`.
///
/// Bundle IDs are 16 hex characters; this produces the short hash suffix used in output
/// filenames (e.g. `bundle.13dc01ac.js`). Used to derive a stable name hash for placeholder
/// bundles, which are never packaged but whose `hash_reference` may appear in other bundles.
pub(super) fn name_hash_for_filename(id: &str) -> String {
  const NAME_HASH_DISPLAY_LEN: usize = 8;
  if id.len() <= NAME_HASH_DISPLAY_LEN {
    id.to_string()
  } else {
    id[id.len() - NAME_HASH_DISPLAY_LEN..].to_string()
  }
}

/// Computes the effective set of non-inline bundle IDs that must be packaged before `bundle`.
///
/// This is a transitive closure through inline bundles: if `bundle` contains an inline bundle,
/// and that inline bundle references bundle B (via a hash_reference placeholder), then B must be
/// packaged before `bundle` runs — because the inline bundle is packaged on-demand inside
/// `bundle`'s packager and needs B's hash to be already resolved.
///
/// The traversal visits inline bundles recursively to handle arbitrarily deep nesting, using a
/// visited set to avoid infinite loops.
pub(super) fn effective_referenced_bundle_ids<B: BundleGraph>(
  bundle: &Bundle,
  graph: &B,
) -> Vec<String> {
  // Filter self-references as defence-in-depth: the JS getReferencedBundles DFS always skips
  // the start node, so self-loop References edges are never returned there. Any implementor of
  // the trait that forgets to do the same would otherwise produce a trivial self-cycle.
  let mut result: Vec<String> = graph
    .get_referenced_bundle_ids(bundle)
    .into_iter()
    .filter(|id| id != &bundle.id)
    .collect();
  let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
  let mut queue: std::collections::VecDeque<String> =
    graph.get_inline_bundle_ids(bundle).into_iter().collect();

  while let Some(inline_id) = queue.pop_front() {
    if !visited.insert(inline_id.clone()) {
      continue;
    }
    if let Some(inline_bundle) = graph.get_bundle_by_id(&inline_id) {
      // Any non-inline bundles the inline bundle references are effective deps of the parent.
      result.extend(
        graph
          .get_referenced_bundle_ids(inline_bundle)
          .into_iter()
          .filter(|id| id != &bundle.id && id != &inline_id),
      );
      // Walk deeper into any inline bundles nested within this inline bundle.
      queue.extend(graph.get_inline_bundle_ids(inline_bundle));
    }
  }

  result
}

/// Extracts one concrete cycle path from the residual graph after Kahn's algorithm has stalled.
///
/// Only nodes with `in_degree > 0` can be part of a cycle (all nodes that were fully resolved
/// have `in_degree == 0`). We restrict the DFS to those nodes and walk the adjacency list until
/// we revisit a node that is already on the current DFS stack — that back-edge gives us the cycle.
///
/// Returns the indices of bundles forming the cycle, starting and ending at the same node so the
/// caller can display it as `A → B → C → A`.
pub(super) fn find_cycle_path(
  bundles: &[Bundle],
  adjacency: &[Vec<usize>],
  in_degree: &[usize],
) -> Vec<usize> {
  let n = bundles.len();
  // Only consider nodes still stuck (in_degree > 0).
  let in_cycle: Vec<bool> = in_degree.iter().map(|&d| d > 0).collect();

  // DFS state:
  //   0 = unvisited
  //   1 = on the current stack (grey)
  //   2 = fully visited (black)
  let mut color = vec![0u8; n];
  // parent[i] tracks which node we came from on the DFS path so we can reconstruct the cycle.
  let mut parent = vec![usize::MAX; n];

  for start in 0..n {
    if !in_cycle[start] || color[start] != 0 {
      continue;
    }

    // Iterative DFS using an explicit stack of (node, edge_iterator_position).
    let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
    color[start] = 1;

    while let Some((node, edge_pos)) = stack.last_mut() {
      let node = *node;
      let neighbors: Vec<usize> = adjacency[node]
        .iter()
        .copied()
        .filter(|&nb| in_cycle[nb])
        .collect();

      if *edge_pos < neighbors.len() {
        let nb = neighbors[*edge_pos];
        *edge_pos += 1;

        if color[nb] == 1 {
          // Back-edge found: nb is the cycle entry point.
          // Reconstruct the path from nb back to nb via `parent` links.
          let cycle_entry = nb;
          let mut path = vec![cycle_entry];
          let mut cur = node;
          while cur != cycle_entry {
            path.push(cur);
            cur = parent[cur];
          }
          path.push(cycle_entry);
          path.reverse();
          return path;
        } else if color[nb] == 0 {
          color[nb] = 1;
          parent[nb] = node;
          stack.push((nb, 0));
        }
      } else {
        color[node] = 2;
        stack.pop();
      }
    }
  }

  // Fallback: return all stuck nodes (should not be reached if the graph truly has a cycle).
  in_degree
    .iter()
    .enumerate()
    .filter_map(|(i, &d)| if d > 0 { Some(i) } else { None })
    .collect()
}

/// Partitions `bundles` into ordered levels such that for every reference edge A → B (bundle A's
/// content embeds bundle B's hash), B appears in an earlier level than A.
///
/// Bundles within the same level are independent of each other and can be processed in parallel.
///
/// # Algorithm (Kahn's BFS topological sort, adapted for level grouping)
///
/// Think of bundles as tasks on a building site where some tasks must finish before others can
/// start. Kahn's algorithm works by repeatedly asking: "which tasks have no remaining
/// prerequisites?" — those form the next batch of work.
///
/// **Step 1 — Count prerequisites.**
/// For every bundle, count how many other bundles it is waiting on (`in_degree`). A bundle with
/// `in_degree == 0` has no prerequisites and is ready to go immediately. We also build the
/// reverse map (`adjacency`): for each bundle, who becomes unblocked once it finishes?
///
/// **Step 2 — Start with the ready set.**
/// All bundles with `in_degree == 0` form level 0. Put them in a queue.
///
/// **Step 3 — Process level by level.**
/// Take everything currently in the queue — that is one level. For each bundle in that level,
/// look at its adjacency list (the bundles it unblocks) and decrement their `in_degree` by one.
/// Any bundle whose `in_degree` just hit zero joins the queue for the *next* level.
///
/// **Step 4 — Repeat until the queue is empty.**
/// Every iteration of the outer loop produces one level. Bundles within a level can be packaged
/// concurrently because they do not depend on each other.
///
/// **Step 5 — Cycle check.**
/// If we finish but some bundles were never enqueued (their `in_degree` never reached zero),
/// those bundles are part of a cycle — there is no valid ordering for them. This is a hard error.
///
/// # Errors
///
/// Returns an error if the reference graph contains a cycle.
#[tracing::instrument(level="info", skip_all, fields(bundle_count = bundles.len()))]
pub(super) fn topological_levels<B: BundleGraph>(
  bundles: &[Bundle],
  graph: &B,
) -> anyhow::Result<Vec<Vec<Bundle>>> {
  // Build an index from bundle ID to position in `bundles`.
  let id_to_idx: HashMap<&str, usize> = bundles
    .iter()
    .enumerate()
    .map(|(i, b)| (b.id.as_str(), i))
    .collect();

  // in_degree[i] = number of bundles that bundle i depends on (i.e. must come before i).
  // adjacency[i] = list of bundle indices that depend on bundle i (i.e. come after i).
  let n = bundles.len();
  let mut in_degree = vec![0usize; n];
  let mut adjacency: Vec<Vec<usize>> = vec![vec![]; n];

  for (i, bundle) in bundles.iter().enumerate() {
    // Collect the effective set of referenced (non-inline) bundle IDs for this bundle.
    // This includes:
    //   1. Bundles directly referenced by this bundle via References edges.
    //   2. Bundles referenced by any inline bundles contained within this bundle,
    //      transitively — because inline bundles are packaged on-demand inside the
    //      parent packager and may embed hash_reference placeholders that must already
    //      be resolved by the time the parent packager runs.
    let effective_refs = effective_referenced_bundle_ids(bundle, graph);

    for ref_id in effective_refs {
      // bundle i references ref_id, meaning ref_id must be packaged before i.
      // So ref_id -> i in the processing order.
      if let Some(&dep_idx) = id_to_idx.get(ref_id.as_str()) {
        adjacency[dep_idx].push(i);
        in_degree[i] += 1;
      }
      // Referenced bundle not in the graph — ignore gracefully.
    }
  }

  // Kahn's algorithm: build levels using BFS from zero-in-degree nodes.
  let mut queue: VecDeque<usize> = in_degree
    .iter()
    .enumerate()
    .filter_map(|(i, &deg)| if deg == 0 { Some(i) } else { None })
    .collect();

  let mut levels: Vec<Vec<Bundle>> = Vec::new();
  let mut processed = 0usize;

  while !queue.is_empty() {
    let level_size = queue.len();
    let mut level: Vec<Bundle> = Vec::with_capacity(level_size);

    for _ in 0..level_size {
      let idx = queue.pop_front().expect("queue non-empty");
      level.push(bundles[idx].clone());
      processed += 1;

      for &dependent_idx in &adjacency[idx] {
        in_degree[dependent_idx] -= 1;
        if in_degree[dependent_idx] == 0 {
          queue.push_back(dependent_idx);
        }
      }
    }

    levels.push(level);
  }

  if processed != n {
    // Some nodes were never enqueued — there is a cycle.
    // Find an actual cycle path for actionable diagnostics.
    let cycle_path = find_cycle_path(bundles, &adjacency, &in_degree);
    let cycle_str = cycle_path
      .iter()
      .map(|&i| {
        let b = &bundles[i];
        match &b.name {
          Some(name) => format!("{} (id: {})", name, b.id),
          None => format!("(id: {})", b.id),
        }
      })
      .collect::<Vec<_>>()
      .join(" → ");
    return Err(anyhow!(
      "Cycle detected in bundle reference graph: {cycle_str}"
    ));
  }

  Ok(levels)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use atlaspack_core::types::BundleBehavior;
  use pretty_assertions::assert_eq;

  use crate::requests::test_utils::bundle_graph::{MockBundleGraph, make_test_bundle};

  use super::*;

  fn make_bundle(id: &str, hash_reference: &str) -> Bundle {
    make_test_bundle(id, hash_reference)
  }

  #[test]
  fn test_topo_single_bundle_no_refs() {
    let bundles = vec![make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa")];
    let graph = MockBundleGraph::builder().bundles(bundles.clone()).build();
    let levels = topological_levels(&bundles, &graph).unwrap();
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0].len(), 1);
    assert_eq!(levels[0][0].id, "a");
  }

  #[test]
  fn test_topo_linear_chain() {
    // c references b, b references a — so order must be: a, b, c
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let c = make_bundle("c", "HASH_REF_cccccccccccccccc");
    let bundles = vec![c.clone(), b.clone(), a.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(bundles.clone())
      .reference("c", "b")
      .reference("b", "a")
      .build();
    let levels = topological_levels(&bundles, &graph).unwrap();
    assert_eq!(levels.len(), 3);
    assert_eq!(levels[0][0].id, "a");
    assert_eq!(levels[1][0].id, "b");
    assert_eq!(levels[2][0].id, "c");
  }

  #[test]
  fn test_topo_diamond() {
    // d references b and c; b references a; c references a
    // Level 0: a
    // Level 1: b, c (parallel)
    // Level 2: d
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let c = make_bundle("c", "HASH_REF_cccccccccccccccc");
    let d = make_bundle("d", "HASH_REF_dddddddddddddddd");
    let bundles = vec![a.clone(), b.clone(), c.clone(), d.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(bundles.clone())
      .reference("b", "a")
      .reference("c", "a")
      .reference("d", "b")
      .reference("d", "c")
      .build();
    let levels = topological_levels(&bundles, &graph).unwrap();
    assert_eq!(levels.len(), 3);
    assert_eq!(levels[0][0].id, "a");
    let mut level1_ids: Vec<&str> = levels[1].iter().map(|b| b.id.as_str()).collect();
    level1_ids.sort();
    assert_eq!(level1_ids, vec!["b", "c"]);
    assert_eq!(levels[2][0].id, "d");
  }

  #[test]
  fn test_topo_disconnected_subgraphs() {
    // a and b have no relationship — both should appear in level 0
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let bundles = vec![a.clone(), b.clone()];
    let graph = MockBundleGraph::builder().bundles(bundles.clone()).build();
    let levels = topological_levels(&bundles, &graph).unwrap();
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0].len(), 2);
  }

  #[test]
  fn test_topo_cycle_is_hard_error() {
    // a references b and b references a — cycle
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let bundles = vec![a.clone(), b.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(bundles.clone())
      .reference("a", "b")
      .reference("b", "a")
      .build();
    let result = topological_levels(&bundles, &graph);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Cycle detected"), "unexpected error: {msg}");
    // Should include a " → " arrow showing the actual cycle path, not just a list.
    assert!(
      msg.contains(" → "),
      "expected cycle path in error, got: {msg}"
    );
  }

  #[test]
  fn test_topo_cycle_three_node_path() {
    // a → b → c → a
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let c = make_bundle("c", "HASH_REF_cccccccccccccccc");
    let bundles = vec![a.clone(), b.clone(), c.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(bundles.clone())
      .reference("a", "b")
      .reference("b", "c")
      .reference("c", "a")
      .build();
    let result = topological_levels(&bundles, &graph);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Cycle detected"), "unexpected error: {msg}");
    // The message must show at least two arrows — i.e. three nodes in the path.
    let arrow_count = msg.matches(" → ").count();
    assert!(
      arrow_count >= 2,
      "expected at least 3-node cycle path in error, got: {msg}"
    );
  }

  #[test]
  fn test_topo_self_reference_is_not_a_cycle() {
    // A bundle that has a References edge pointing to itself should not be treated as a cycle.
    // This mirrors the JS getReferencedBundles guard (node.value.id === bundle.id → skip).
    // In practice this occurs when a bundle name embeds its own HASH_REF placeholder.
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let bundles = vec![a.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(bundles.clone())
      .reference("a", "a") // self-loop
      .build();
    let levels = topological_levels(&bundles, &graph).expect("self-loop should not be a cycle");
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0][0].id, "a");
  }

  #[test]
  fn test_topo_inline_bundle_transitive_deps_respected() {
    // HTML references inline script (filtered from main loop).
    // Inline script references JS bundle B.
    // Effective deps of HTML = {B}, so B must be in an earlier level than HTML.
    //
    //  B (normal)  <-- inline script references B
    //  inline (inlineIsolated, filtered out of main loop)
    //  HTML (references inline, which references B)
    //
    // Expected levels: [B], [HTML]
    let b = make_bundle("b", "HASH_REF_bbbbbbbbbbbbbbbb");
    let mut inline_bundle = make_bundle("inline", "HASH_REF_iiiiiiiiiiiiiiii");
    inline_bundle.bundle_behavior = Some(BundleBehavior::InlineIsolated);
    let html = make_bundle("html", "HASH_REF_hhhhhhhhhhhhhhhh");

    // Only non-inline bundles appear in the main loop (filtered by PackagingRequest::run()).
    // The inline bundle must still be present in the graph so get_bundle_by_id can find it
    // when the topo sort walks transitively through inline children.
    let bundles = vec![b.clone(), html.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(vec![b.clone(), inline_bundle.clone(), html.clone()])
      // inline script references b's hash
      .reference("inline", "b")
      // html contains the inline bundle
      .inline_child("html", "inline")
      .build();

    let levels = topological_levels(&bundles, &graph).unwrap();
    assert_eq!(levels.len(), 2, "expected 2 levels, got: {levels:?}");
    assert_eq!(levels[0][0].id, "b", "b must be packaged first");
    assert_eq!(levels[1][0].id, "html", "html must come after b");
  }

  #[test]
  fn test_topo_hash_ref_unknown_bundle_ignored() {
    // A bundle references an ID not in the graph — should not error, just ignore it.
    let a = make_bundle("a", "HASH_REF_aaaaaaaaaaaaaaaa");
    let bundles = vec![a.clone()];
    let graph = MockBundleGraph::builder()
      .bundles(bundles.clone())
      .reference("a", "nonexistent")
      .build();
    let levels = topological_levels(&bundles, &graph).unwrap();
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0][0].id, "a");
  }
}
