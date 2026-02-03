use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
  asset_graph::{AssetGraph, AssetGraphNode},
  types::{Asset, Dependency, Symbol},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SymbolTracker {
  requirements_by_dep: HashMap<String, Vec<SymbolRequirement>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolRequirement {
  pub symbol: Symbol,
  pub final_location: Option<FinalSymbolLocation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinalSymbolLocation {
  pub local_name: String,
  pub imported_name: String,
  pub providing_asset: Arc<Asset>,
}

fn dep_has_symbol(dep: &Dependency, symbol: &Symbol) -> bool {
  dep
    .symbols
    .as_ref()
    .is_some_and(|dep_symbols| dep_symbols.iter().any(|s| s.exported == symbol.exported))
}

fn get_parent_asset_node_id<'a>(
  asset_graph: &'a AssetGraph,
  dep: &'a Dependency,
) -> anyhow::Result<usize> {
  let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(dep.id.as_str()) else {
    return Err(anyhow!(
      "Unable to get node ID for dependency ID {}",
      dep.id
    ));
  };

  let dep_node_incoming_neighbors = asset_graph.get_incoming_neighbor_node_ids(dep_node_id);

  let [parent_asset_node_id] = dep_node_incoming_neighbors.as_slice() else {
    return Err(anyhow!(
      "Dependency {} does not have exactly one parent asset",
      dep.id
    ));
  };

  Ok(*parent_asset_node_id)
}

fn get_incoming_dependencies<'a>(
  asset_graph: &'a AssetGraph,
  asset_node_id: &usize,
) -> Vec<&'a Dependency> {
  let incoming_nodes = asset_graph.get_incoming_neighbors(asset_node_id);
  incoming_nodes
    .iter()
    .filter_map(|node| {
      if let AssetGraphNode::Dependency(dep) = node {
        Some(dep.as_ref())
      } else {
        None
      }
    })
    .collect()
}

impl SymbolTracker {
  /// Handles the tracking of symbols for a given asset.
  ///
  /// This looks at both the dependencies this asset has, to register symbols
  /// that it needs to find, and the symbols that this asset provides, to
  /// satisfy any outstanding requirements from above.
  pub fn track_symbols(
    &mut self,
    asset_graph: &AssetGraph,
    asset: &Arc<Asset>,
    dependencies: &[Dependency],
    // TODO: This should actually return a list of the new requests that need to
    // be kicked off, what `propagate_requested_symbols` does at the moment
  ) -> anyhow::Result<()> {
    // Track all of the symbols that are being requested by dependencies of this
    // asset
    for dep in dependencies {
      let Some(symbols) = &dep.symbols else {
        continue;
      };

      for symbol in symbols {
        self.add_required_symbol(
          &dep.id,
          SymbolRequirement {
            symbol: symbol.clone(),
            final_location: None,
          },
        );
      }
    }

    // Track all of the symbols that this asset provides to other assets, and
    // satisfy any requirements that have been asked about them
    if let Some(provided_symbols) = &asset.symbols {
      let Some(asset_node_id) = asset_graph.get_node_id_by_content_key(asset.id.as_str()) else {
        return Err(anyhow!("Unable to get node ID for asset ID {}", asset.id));
      };

      let incoming_nodes = asset_graph.get_incoming_neighbors(asset_node_id);
      let incoming_deps = incoming_nodes.iter().filter_map(|node| match node {
        AssetGraphNode::Dependency(dep) => Some(dep.as_ref()),
        _ => None,
      });

      for incoming_dep in incoming_deps {
        for sym in provided_symbols {
          // If the incoming dependency does not ask for this symbol, skip it
          if !dep_has_symbol(incoming_dep, sym) {
            continue;
          }

          // If a symbol is weak, it cannot be a final location. We still add
          // these as requirements, so that other assets asking for the symbol
          // can find it here, but we don't want to register this re-export as
          // the source of truth
          if sym.is_weak {
            continue;
          }

          let final_location = FinalSymbolLocation {
            local_name: sym.local.clone(),
            imported_name: sym.exported.clone(),
            providing_asset: asset.clone(),
          };

          self.handle_answer_propagation(asset_graph, final_location, incoming_dep)?;
        }
      }
    }

    Ok(())
  }

  fn handle_answer_propagation(
    &mut self,
    asset_graph: &AssetGraph,
    located_symbol: FinalSymbolLocation,
    dep: &Dependency,
  ) -> anyhow::Result<()> {
    let mut work_queue = vec![dep];

    while let Some(dep) = work_queue.pop() {
      // Process the requirements for this dependency
      let Some(required_symbols) = self.requirements_by_dep.get_mut(&dep.id) else {
        continue;
      };

      for required in required_symbols {
        if required.symbol.exported != located_symbol.imported_name {
          continue;
        }

        // If the requirement is already satisfied, and we have a new location that
        // matches, we have a conflict
        if required.final_location.is_some() {
          return Err(anyhow!(
            "Required symbol {} for dep [{}] has already been located",
            required.symbol.exported,
            dep.id
          ));
        } else {
          required.final_location = Some(located_symbol.clone());
        }

        // If we satisfied this requirement, we need to propagate the location
        // up to the parent asset's dependencies as well
        let parent_asset_node_id = get_parent_asset_node_id(asset_graph, dep)?;
        let parent_asset_dependencies =
          get_incoming_dependencies(asset_graph, &parent_asset_node_id);

        work_queue.extend_from_slice(parent_asset_dependencies.as_slice());
      }
    }

    Ok(())
  }

  fn add_required_symbol(&mut self, dep_id: &str, question: SymbolRequirement) {
    self
      .requirements_by_dep
      .entry(dep_id.to_string())
      .or_default()
      .push(question);
  }
}
