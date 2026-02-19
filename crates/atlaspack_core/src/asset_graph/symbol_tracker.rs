use anyhow::anyhow;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
  asset_graph::{AssetGraph, AssetGraphNode},
  types::{Asset, AssetId, Dependency, DependencyId, Symbol},
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

  /// Finalizes the symbol tracker, returning a read-only version of the
  /// FinalizedSymbolTracker that can be used for serialization and lookup of
  /// symbol locations. This method also validates that there are no outstanding
  /// symbol requirements that have not been satisfied, and that there are no
  /// conflicts.
  #[tracing::instrument(name = "finalize_symbol_tracker", skip_all)]
  pub fn finalize(self) -> FinalizedSymbolTracker {
    let mut requirements_by_dep: HashMap<DependencyId, DependencyUsedSymbols> = HashMap::new();

    for (dep_id, requirements) in self.requirements_by_dep.into_iter() {
      let mut used_symbols = HashMap::new();

      for requirement in requirements.into_iter() {
        let Some(final_location) = requirement.final_location else {
          panic!(
            "Symbol {} required by dependency [{}] was not satisfied",
            requirement.symbol.exported, dep_id
          );
        };

        used_symbols.insert(
          requirement.symbol.clone(),
          UsedSymbol {
            symbol: requirement.symbol,
            asset: final_location.providing_asset.id.clone(),
          },
        );
      }

      requirements_by_dep.insert(dep_id, used_symbols);
    }

    FinalizedSymbolTracker {
      requirements_by_dep,
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UsedSymbol {
  pub symbol: Symbol,
  pub asset: AssetId,
}

pub type DependencyUsedSymbols = HashMap<Symbol, UsedSymbol>;

#[derive(Clone, Debug, PartialEq)]
pub struct FinalizedSymbolTracker {
  requirements_by_dep: HashMap<DependencyId, DependencyUsedSymbols>,
}

impl FinalizedSymbolTracker {
  pub fn get_used_symbols_for_dependency(
    &self,
    dep_id: &DependencyId,
  ) -> Option<&DependencyUsedSymbols> {
    self.requirements_by_dep.get(dep_id)
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;
  use std::sync::atomic::{AtomicU32, Ordering};

  use pretty_assertions::assert_eq;

  use super::*;
  use crate::types::{DependencyBuilder, Environment, Priority, SpecifierType, Target};

  static ASSET_COUNTER: AtomicU32 = AtomicU32::new(0);

  type TestSymbol<'a> = (&'a str, &'a str, bool);

  fn symbol(test_symbol: &TestSymbol) -> Symbol {
    let (local, exported, is_weak) = test_symbol;
    Symbol {
      local: String::from(*local),
      exported: String::from(*exported),
      is_weak: *is_weak,
      ..Symbol::default()
    }
  }

  fn make_asset(file_path: &str, symbols: Vec<TestSymbol>) -> Arc<Asset> {
    let unique_id = ASSET_COUNTER.fetch_add(1, Ordering::SeqCst);
    Arc::new(Asset {
      id: format!("asset_{:016x}", unique_id),
      file_path: PathBuf::from(file_path),
      symbols: Some(symbols.iter().map(symbol).collect()),
      ..Asset::default()
    })
  }

  fn make_dependency(
    source_asset: &Asset,
    specifier: &str,
    symbols: Vec<TestSymbol>,
  ) -> Dependency {
    DependencyBuilder::default()
      .symbols(symbols.iter().map(symbol).collect())
      .specifier(specifier.to_string())
      .env(Arc::new(Environment::default()))
      .specifier_type(SpecifierType::default())
      .source_path(source_asset.file_path.clone())
      .source_asset_id(source_asset.id.clone())
      .priority(Priority::default())
      .build()
  }

  fn make_entry_dependency() -> Dependency {
    Dependency::entry(String::from("entry.js"), Target::default())
  }

  /// Creates a simple graph: entry_dep -> entry_asset -> dep_a -> asset_a
  fn setup_simple_graph() -> (AssetGraph, Arc<Asset>, Dependency, Arc<Asset>) {
    let mut graph = AssetGraph::new();

    // Entry dependency
    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    // Entry asset that imports symbol "a" from "./a.js"
    let entry_asset = make_asset("entry.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    // Dependency on "./a.js" requesting symbol "a"
    let dep_a = make_dependency(&entry_asset, "./a.js", vec![("a", "a", false)]);
    let dep_a_node = graph.add_dependency(dep_a.clone(), false);
    graph.add_edge(&entry_asset_node, &dep_a_node);

    // Asset a.js that provides symbol "a"
    let asset_a = make_asset("a.js", vec![("a", "a", false)]);
    let asset_a_node = graph.add_asset(asset_a.clone(), false);
    graph.add_edge(&dep_a_node, &asset_a_node);

    (graph, entry_asset, dep_a, asset_a)
  }

  #[test]
  fn track_symbols_registers_requirements_from_dependencies() {
    let (graph, entry_asset, dep_a, _asset_a) = setup_simple_graph();
    let mut tracker = SymbolTracker::default();

    // Track symbols for entry_asset with its dependency
    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a.clone()])
      .unwrap();

    // Verify the requirement was registered
    let requirements = tracker.requirements_by_dep.get(&dep_a.id).unwrap();
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].symbol.exported, "a");
    assert!(
      requirements[0].final_location.is_none(),
      "Symbol should not be resolved yet"
    );
  }

  #[test]
  fn track_symbols_satisfies_requirements_when_asset_provides_symbol() {
    let (graph, entry_asset, dep_a, asset_a) = setup_simple_graph();
    let mut tracker = SymbolTracker::default();

    // First, register the requirement from the entry asset
    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a.clone()])
      .unwrap();

    // Now track the providing asset - this should satisfy the requirement
    tracker.track_symbols(&graph, &asset_a, &[]).unwrap();

    // Verify the requirement is now satisfied
    let requirements = tracker.requirements_by_dep.get(&dep_a.id).unwrap();
    assert_eq!(requirements.len(), 1);

    let final_location = requirements[0]
      .final_location
      .as_ref()
      .expect("Symbol should be resolved");
    assert_eq!(final_location.providing_asset.id, asset_a.id);
    assert_eq!(final_location.local_name, "a");
    assert_eq!(final_location.imported_name, "a");
  }

  #[test]
  fn track_symbols_does_not_satisfy_with_weak_symbols() {
    let mut graph = AssetGraph::new();

    // Entry dependency
    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    // Entry asset
    let entry_asset = make_asset("entry.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    // Dependency requesting symbol "a"
    let dep_a = make_dependency(&entry_asset, "./lib.js", vec![("a", "a", false)]);
    let dep_a_node = graph.add_dependency(dep_a.clone(), false);
    graph.add_edge(&entry_asset_node, &dep_a_node);

    // Library asset that re-exports "a" (weak symbol - is_weak: true)
    let lib_asset = make_asset("lib.js", vec![("a", "a", true)]);
    let lib_asset_node = graph.add_asset(lib_asset.clone(), false);
    graph.add_edge(&dep_a_node, &lib_asset_node);

    let mut tracker = SymbolTracker::default();

    // Register requirement
    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a.clone()])
      .unwrap();

    // Track the library asset with weak symbol - should NOT satisfy the requirement
    tracker.track_symbols(&graph, &lib_asset, &[]).unwrap();

    // Verify the requirement is still unsatisfied
    let requirements = tracker.requirements_by_dep.get(&dep_a.id).unwrap();
    assert!(
      requirements[0].final_location.is_none(),
      "Weak symbol should not satisfy requirement"
    );
  }

  #[test]
  fn track_symbols_propagates_answer_up_dependency_chain() {
    let mut graph = AssetGraph::new();

    // entry_dep -> entry_asset -> lib_dep -> lib_asset -> a_dep -> a_asset
    // Entry requests "a", lib re-exports "a", a.js provides "a"

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let entry_asset = make_asset("entry.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    let lib_dep = make_dependency(&entry_asset, "./lib.js", vec![("a", "a", false)]);
    let lib_dep_node = graph.add_dependency(lib_dep.clone(), false);
    graph.add_edge(&entry_asset_node, &lib_dep_node);

    // lib.js re-exports "a" (weak)
    let lib_asset = make_asset("lib.js", vec![("a", "a", true)]);
    let lib_asset_node = graph.add_asset(lib_asset.clone(), false);
    graph.add_edge(&lib_dep_node, &lib_asset_node);

    let a_dep = make_dependency(&lib_asset, "./a.js", vec![("a", "a", false)]);
    let a_dep_node = graph.add_dependency(a_dep.clone(), false);
    graph.add_edge(&lib_asset_node, &a_dep_node);

    // a.js provides "a" (strong symbol)
    let a_asset = make_asset("a.js", vec![("a", "a", false)]);
    let a_asset_node = graph.add_asset(a_asset.clone(), false);
    graph.add_edge(&a_dep_node, &a_asset_node);

    let mut tracker = SymbolTracker::default();

    // Register requirements in traversal order
    tracker
      .track_symbols(&graph, &entry_asset, &[lib_dep.clone()])
      .unwrap();
    tracker
      .track_symbols(&graph, &lib_asset, &[a_dep.clone()])
      .unwrap();

    // Now the providing asset satisfies the requirement
    tracker.track_symbols(&graph, &a_asset, &[]).unwrap();

    // Both lib_dep and a_dep should now have the requirement satisfied
    let lib_requirements = tracker.requirements_by_dep.get(&lib_dep.id).unwrap();
    let lib_final = lib_requirements[0]
      .final_location
      .as_ref()
      .expect("lib_dep requirement should be satisfied");
    assert_eq!(
      lib_final.providing_asset.id, a_asset.id,
      "lib_dep should point to a.js as provider"
    );

    let a_requirements = tracker.requirements_by_dep.get(&a_dep.id).unwrap();
    let a_final = a_requirements[0]
      .final_location
      .as_ref()
      .expect("a_dep requirement should be satisfied");
    assert_eq!(
      a_final.providing_asset.id, a_asset.id,
      "a_dep should point to a.js as provider"
    );
  }

  #[test]
  fn finalize_creates_correct_used_symbols_mapping() {
    let (graph, entry_asset, dep_a, asset_a) = setup_simple_graph();
    let mut tracker = SymbolTracker::default();

    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a.clone()])
      .unwrap();
    tracker.track_symbols(&graph, &asset_a, &[]).unwrap();

    let finalized = tracker.finalize();

    let used_symbols = finalized
      .get_used_symbols_for_dependency(&dep_a.id)
      .expect("Should have used symbols for dep_a");

    assert_eq!(used_symbols.len(), 1);

    let symbol_key = symbol(&("a", "a", false));
    let used_symbol = used_symbols
      .get(&symbol_key)
      .expect("Should have symbol 'a'");
    assert_eq!(used_symbol.asset, asset_a.id);
    assert_eq!(used_symbol.symbol.exported, "a");
  }

  #[test]
  #[should_panic(expected = "was not satisfied")]
  fn finalize_panics_on_unsatisfied_requirements() {
    let (graph, entry_asset, dep_a, _asset_a) = setup_simple_graph();
    let mut tracker = SymbolTracker::default();

    // Register requirement but never satisfy it
    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a])
      .unwrap();

    // This should panic because the requirement is not satisfied
    tracker.finalize();
  }

  #[test]
  fn track_symbols_errors_on_duplicate_symbol_resolution() {
    let (graph, entry_asset, dep_a, asset_a) = setup_simple_graph();
    let mut tracker = SymbolTracker::default();

    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a])
      .unwrap();

    // Satisfy once
    tracker.track_symbols(&graph, &asset_a, &[]).unwrap();

    // Trying to satisfy again should error
    let result = tracker.track_symbols(&graph, &asset_a, &[]);
    assert!(
      result.is_err(),
      "Should error when symbol is resolved twice"
    );
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("has already been located")
    );
  }
}
