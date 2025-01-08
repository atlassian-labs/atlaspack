use std::collections::HashSet;
use std::sync::Arc;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::types::Asset;
use crate::types::Dependency;
use crate::types::Symbol;

use super::asset_graph::DependencyState;
use super::asset_graph::{AssetGraph, DependencyNode};

const CHAR_STAR: &str = "*";

/// Propagates the requested symbols from an incoming dependency to an asset,
/// and forwards those symbols to re-exported dependencies if needed.
/// This may result in assets becoming un-deferred and transformed if they
/// now have requested symbols.
pub fn propagate_requested_symbols<F>(
  asset_graph: &mut AssetGraph,
  initial_asset_idx: NodeIndex,
  initial_dependency_idx: NodeIndex,
  on_undeferred: &mut F,
) where
  F: FnMut(NodeIndex, Arc<Dependency>),
{
  let mut next = vec![(initial_asset_idx, initial_dependency_idx)];

  while let Some((asset_idx, dependency_idx)) = next.pop() {
    let mut dependency_re_exports = HashSet::<String>::default();
    let mut dependency_wildcards = HashSet::<String>::default();
    let mut asset_requested_symbols_buf = HashSet::<String>::default();

    let dependency_node = asset_graph.get_dependency_node(&dependency_idx).unwrap();
    let asset_node = asset_graph.get_asset_node(&asset_idx).unwrap();

    if dependency_node.requested_symbols.contains(CHAR_STAR) {
      // If the requested symbols includes the "*" namespace, we
      // need to include all of the asset's exported symbols.
      if let Some(symbols) = &asset_node.asset.symbols {
        for sym in symbols {
          if !asset_node.requested_symbols.contains(&sym.exported) {
            continue;
          }
          asset_requested_symbols_buf.insert(sym.exported.clone());
          if !sym.is_weak {
            continue;
          }
          // Propagate re-exported symbol to dependency.
          dependency_re_exports.insert(sym.local.clone());
        }
      }

      // Propagate to all export * wildcard dependencies.
      dependency_wildcards.insert(CHAR_STAR.to_string());
    } else {
      // Otherwise, add each of the requested symbols to the asset.
      for sym in dependency_node.requested_symbols.iter() {
        if asset_node.requested_symbols.contains(sym) {
          continue;
        }
        asset_requested_symbols_buf.insert(sym.clone());

        let Some(asset_symbol) = get_symbol_by_name(&asset_node.asset, sym) else {
          // If symbol wasn't found in the asset or a named re-export.
          // This means the symbol is in one of the export * wildcards, but we don't know
          // which one yet, so we propagate it to _all_ wildcard dependencies.
          dependency_wildcards.insert(sym.clone());
          continue;
        };

        if !asset_symbol.is_weak {
          continue;
        }

        // If the asset exports this symbol
        // Propagate re-exported symbol to dependency.
        dependency_re_exports.insert(asset_symbol.local.clone());
      }
    }

    // Add dependencies to asset
    asset_graph
      .get_asset_node_mut(&asset_idx)
      .unwrap()
      .requested_symbols
      .extend(asset_requested_symbols_buf);

    let deps: Vec<_> = asset_graph
      .graph
      .neighbors_directed(asset_idx, Direction::Outgoing)
      .collect();

    for nested_dependency_idx in deps {
      let mut updated = false;

      {
        let DependencyNode {
          dependency,
          requested_symbols,
          state: _,
        } = asset_graph
          .get_dependency_node_mut(&nested_dependency_idx)
          .unwrap();

        if let Some(symbols) = &dependency.symbols {
          for sym in symbols {
            if sym.is_weak {
              // This is a re-export. If it is a wildcard, add all unmatched symbols
              // to this dependency, otherwise attempt to match a named re-export.
              if sym.local == "*" {
                for wildcard in &dependency_wildcards {
                  if requested_symbols.insert(wildcard.clone()) {
                    updated = true;
                  }
                }
              } else if dependency_re_exports.contains(&sym.local)
                && requested_symbols.insert(sym.exported.clone())
              {
                updated = true;
              }
            } else if requested_symbols.insert(sym.exported.clone()) {
              // This is a normal import. Add the requested symbol.
              updated = true;
            }
          }
        }
      }

      let DependencyNode {
        dependency,
        requested_symbols: _,
        state,
      } = asset_graph
        .get_dependency_node(&nested_dependency_idx)
        .unwrap();

      // If the dependency was updated, propagate to the target asset if there is one,
      // or un-defer this dependency so we transform the requested asset.
      // We must always resolve new dependencies to determine whether they have side effects.
      if updated || *state == DependencyState::New {
        let Some(resolved) = asset_graph
          .graph
          .edges_directed(nested_dependency_idx, Direction::Outgoing)
          .next()
        else {
          on_undeferred(nested_dependency_idx, Arc::clone(dependency));
          continue;
        };
        if resolved.target() == asset_idx {
          continue;
        }
        next.push((resolved.target(), nested_dependency_idx))
      }
    }
  }
}

fn get_symbol_by_name<'a>(asset: &'a Asset, sym: &str) -> Option<&'a Symbol> {
  asset
    .symbols
    .as_ref()
    .and_then(|symbols| symbols.iter().find(|s| s.exported == *sym))
}
