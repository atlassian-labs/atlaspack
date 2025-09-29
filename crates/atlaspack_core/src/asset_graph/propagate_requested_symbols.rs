use std::collections::HashSet;
use std::sync::Arc;

use crate::types::Asset;
use crate::types::Dependency;
use crate::types::Symbol;

use super::asset_graph::AssetGraph;
use super::asset_graph::DependencyState;

const CHAR_STAR: &str = "*";

type NodeId = usize;

/// Propagates the requested symbols from an incoming dependency to an asset,
/// and forwards those symbols to re-exported dependencies if needed.
/// This may result in assets becoming un-deferred and transformed if they
/// now have requested symbols.
pub fn propagate_requested_symbols<F>(
  asset_graph: &mut AssetGraph,
  initial_asset_id: NodeId,
  initial_dependency_id: NodeId,
  on_undeferred: &mut F,
) where
  F: FnMut(NodeId, Arc<Dependency>),
{
  let mut next = vec![(initial_asset_id, initial_dependency_id)];

  while let Some((asset_id, dependency_id)) = next.pop() {
    let mut dependency_re_exports = HashSet::<String>::default();
    let mut dependency_wildcards = HashSet::<String>::default();
    let mut asset_requested_symbols_buf = HashSet::<String>::default();

    let dependency = asset_graph.get_dependency(&dependency_id).unwrap();
    let asset = asset_graph.get_asset(&asset_id).unwrap();

    let asset_symbols = asset_graph.get_requested_symbols(&asset_id).unwrap();
    let dependency_symbols = asset_graph.get_requested_symbols(&dependency_id).unwrap();

    if dependency_symbols.contains(CHAR_STAR) {
      // If the requested symbols includes the "*" namespace, we
      // need to include all of the asset's exported symbols.
      if let Some(symbols) = &asset.symbols {
        for sym in symbols {
          if !asset_symbols.contains(&sym.exported) {
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
      for sym in dependency_symbols.iter() {
        if asset_symbols.contains(sym) {
          continue;
        }
        asset_requested_symbols_buf.insert(sym.clone());

        let Some(asset_symbol) = get_symbol_by_name(asset, sym) else {
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
      .get_requested_symbols_mut(&asset_id)
      .unwrap()
      .extend(asset_requested_symbols_buf);

    let dep_ids = asset_graph.get_outgoing_dependencies(&asset_id);

    for nested_dependency_id in dep_ids {
      let mut updated = false;

      {
        if let Some(symbols) = &dependency.symbols {
          for sym in symbols {
            if sym.is_weak {
              // This is a re-export. If it is a wildcard, add all unmatched symbols
              // to this dependency, otherwise attempt to match a named re-export.
              if sym.local == "*" {
                for wildcard in &dependency_wildcards {
                  if asset_graph.set_requested_symbol(&nested_dependency_id, wildcard.clone()) {
                    updated = true;
                  }
                }
              } else if dependency_re_exports.contains(&sym.local)
                && asset_graph.set_requested_symbol(&nested_dependency_id, sym.exported.clone())
              {
                updated = true;
              }
            } else if asset_graph.set_requested_symbol(&nested_dependency_id, sym.exported.clone())
            {
              // This is a normal import. Add the requested symbol.
              updated = true;
            }
          }
        }
      }

      let state = asset_graph.get_dependency_state(&nested_dependency_id);
      let nested_dependency = asset_graph.get_dependency(&nested_dependency_id).unwrap();

      // If the dependency was updated, propagate to the target asset if there is one,
      // or un-defer this dependency so we transform the requested asset.
      // We must always resolve new dependencies to determine whether they have side effects.
      if updated || *state == DependencyState::New {
        let outgoing_deps = asset_graph.get_outgoing_dependencies(&nested_dependency_id);
        let Some(resolved_asset_id) = outgoing_deps.first() else {
          on_undeferred(nested_dependency_id, nested_dependency.clone());
          continue;
        };

        if *resolved_asset_id == asset_id {
          continue;
        }

        next.push((*resolved_asset_id, nested_dependency_id))
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
