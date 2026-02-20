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
  /// If true, this requirement was speculatively forwarded through a star re-export
  /// and doesn't need to be satisfied (another star dep might satisfy it instead)
  pub is_speculative: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinalSymbolLocation {
  /// The local name in the providing asset (used for scope hoisting)
  pub local_name: String,
  /// The exported/imported name from the providing asset
  pub imported_name: String,
  /// The ID of the asset that provides this symbol (cheaper than Arc<Asset>)
  pub providing_asset_id: AssetId,
}

fn dep_has_symbol(dep: &Dependency, symbol: &Symbol) -> bool {
  dep.symbols.as_ref().is_some_and(|dep_symbols| {
    dep_symbols
      .iter()
      .any(|s| s.exported == symbol.exported || is_star_reexport_symbol(s))
  })
}

/// Returns true if this symbol represents a star re-export (`export * from './dep'`).
/// Star re-exports have both `exported` and `local` set to "*".
fn is_star_reexport_symbol(symbol: &Symbol) -> bool {
  symbol.exported == "*" && symbol.local == "*"
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

/// Given an asset and a mangled local symbol name from a dependency, find the
/// exported name that corresponds to it in the asset's symbols.
///
/// For a re-export like `export {foo as renamedFoo} from './foo'`:
/// - The dependency to foo.js has: local='$assetId$re_export$renamedFoo', exported='foo'
/// - The barrel asset has: local='$assetId$re_export$renamedFoo', exported='renamedFoo'
///
/// So we match on the mangled local name to find the exported name.
fn find_exported_name_for_local(asset: &Asset, mangled_local: &str) -> Option<String> {
  let symbols = asset.symbols.as_ref()?;
  for sym in symbols {
    if sym.local == mangled_local {
      return Some(sym.exported.clone());
    }
  }
  None
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
    // Collect symbols requested from this asset by incoming dependencies
    let incoming_requested_symbols = self.get_incoming_requested_symbols(asset_graph, asset);

    // Track all of the symbols that are being requested by dependencies of this
    // asset
    for dep in dependencies {
      let Some(symbols) = &dep.symbols else {
        continue;
      };

      // Check if this dependency is a star re-export
      let is_star_dep = symbols.iter().any(is_star_reexport_symbol);

      for symbol in symbols {
        // Skip any "*" symbols - they are markers for namespace exports or star re-exports,
        // not actual requirements that need to be satisfied
        if symbol.exported == "*" {
          continue;
        }
        self.add_required_symbol(
          &dep.id,
          SymbolRequirement {
            symbol: symbol.clone(),
            final_location: None,
            is_speculative: false,
          },
        );
      }

      // For star re-export dependencies, propagate all symbols requested from
      // this asset down to the star dependency. This is how `export * from './dep'`
      // forwards symbol requests to the target module.
      if is_star_dep {
        for requested_symbol in &incoming_requested_symbols {
          // Don't add duplicate requirements
          if symbols.iter().any(|s| s.exported == *requested_symbol) {
            continue;
          }
          // Create a forwarded requirement for this symbol
          // Mark as speculative since this star dep might not provide this symbol
          // (another star dep sibling might provide it instead)
          self.add_required_symbol(
            &dep.id,
            SymbolRequirement {
              symbol: Symbol {
                exported: requested_symbol.clone(),
                local: "*".to_string(), // Mark as coming from star re-export
                is_weak: true,
                ..Default::default()
              },
              final_location: None,
              is_speculative: true,
            },
          );
        }
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
          // Skip star symbols on the asset - these are markers for "export *" patterns,
          // not actual symbols that can satisfy requirements
          if sym.exported == "*" {
            continue;
          }

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
            providing_asset_id: asset.id.clone(),
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
    // Work queue contains (dependency, symbol_name_to_match)
    // The symbol_name_to_match is the exported name we're looking for at this level
    let mut work_queue = vec![(dep, located_symbol.imported_name.clone())];

    // Reusable buffer for matched requirements to avoid repeated allocations
    let mut matched_locals: Vec<String> = Vec::new();

    while let Some((dep, symbol_name_to_match)) = work_queue.pop() {
      matched_locals.clear();

      // Process the requirements for this dependency
      {
        let Some(required_symbols) = self.requirements_by_dep.get_mut(&dep.id) else {
          continue;
        };

        for required in required_symbols.iter_mut() {
          if required.symbol.exported != symbol_name_to_match {
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
          }

          required.final_location = Some(located_symbol.clone());
          // Store the local name for propagation lookup
          matched_locals.push(required.symbol.local.clone());
        }
      }

      // Now propagate for matched requirements (outside the mutable borrow)
      for symbol_local in &matched_locals {
        // If we satisfied this requirement, we need to propagate the location
        // up to the parent asset's dependencies as well
        let parent_asset_node_id = get_parent_asset_node_id(asset_graph, dep)?;

        // Get the parent asset to check for re-export mappings (local -> exported name)
        let parent_asset_node = asset_graph.get_node(&parent_asset_node_id);

        // For each incoming dependency of the parent asset, we need to determine
        // what symbol name to look for. If the parent asset has a symbol that
        // maps the dependency's mangled local name to an exported name, use that.
        let parent_asset_dependencies =
          get_incoming_dependencies(asset_graph, &parent_asset_node_id);

        for parent_dep in parent_asset_dependencies {
          // For star re-exports (local == "*"), the symbol passes through unchanged,
          // so we use the same symbol name. For explicit re-exports, we look up the
          // exported name via the mangled local name mapping.
          let symbol_for_parent = if symbol_local == "*" {
            // Star re-export: symbol name passes through unchanged
            symbol_name_to_match.clone()
          } else if let Some(AssetGraphNode::Asset(asset)) = parent_asset_node {
            find_exported_name_for_local(asset, symbol_local)
              .unwrap_or_else(|| symbol_name_to_match.clone())
          } else {
            symbol_name_to_match.clone()
          };

          work_queue.push((parent_dep, symbol_for_parent));
        }
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

  /// Collects all symbols that are being requested from this asset by incoming dependencies.
  /// This looks at both:
  /// 1. Requirements that have been registered on incoming dependencies but not yet satisfied
  /// 2. Symbols declared on the incoming dependency itself (from the dependency's symbols field)
  /// This is used to propagate symbol requests through star re-exports.
  fn get_incoming_requested_symbols(
    &self,
    asset_graph: &AssetGraph,
    asset: &Arc<Asset>,
  ) -> Vec<String> {
    let Some(asset_node_id) = asset_graph.get_node_id_by_content_key(asset.id.as_str()) else {
      return Vec::new();
    };

    let incoming_deps = get_incoming_dependencies(asset_graph, &asset_node_id);
    let mut requested_symbols = Vec::new();

    for dep in incoming_deps {
      // Look at requirements that have been registered on this dependency
      if let Some(requirements) = self.requirements_by_dep.get(&dep.id) {
        for req in requirements {
          // Skip if already satisfied
          if req.final_location.is_some() {
            continue;
          }
          // Skip the star symbol itself
          if req.symbol.exported == "*" {
            continue;
          }
          if !requested_symbols.contains(&req.symbol.exported) {
            requested_symbols.push(req.symbol.exported.clone());
          }
        }
      }

      // Also look at symbols declared on the dependency itself
      // This handles the case where the dependency hasn't been processed yet
      // but its symbols are already known
      if let Some(symbols) = &dep.symbols {
        for sym in symbols {
          // Skip the star symbol itself
          if sym.exported == "*" {
            continue;
          }
          if !requested_symbols.contains(&sym.exported) {
            requested_symbols.push(sym.exported.clone());
          }
        }
      }
    }

    requested_symbols
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
        // Skip "*" symbols in finalization - these are namespace markers, not actual
        // symbols that need to be satisfied
        if requirement.symbol.exported == "*" {
          continue;
        }

        let Some(final_location) = requirement.final_location else {
          // Speculative requirements (from star re-exports) don't need to be satisfied
          // since another sibling star dep might satisfy them instead
          if requirement.is_speculative {
            continue;
          }
          panic!(
            "Symbol {} required by dependency [{}] was not satisfied",
            requirement.symbol.exported, dep_id
          );
        };

        used_symbols.insert(
          requirement.symbol.clone(),
          UsedSymbol {
            symbol: requirement.symbol,
            asset: final_location.providing_asset_id,
            resolved_symbol: final_location.imported_name,
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
  /// The original requested symbol (from the dependency)
  pub symbol: Symbol,
  /// The asset that provides this symbol
  pub asset: AssetId,
  /// The resolved symbol name in the providing asset (may differ from symbol.exported due to renames)
  pub resolved_symbol: String,
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
      .track_symbols(&graph, &entry_asset, std::slice::from_ref(&dep_a))
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
      .track_symbols(&graph, &entry_asset, std::slice::from_ref(&dep_a))
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
    assert_eq!(final_location.providing_asset_id, asset_a.id);
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
      .track_symbols(&graph, &entry_asset, std::slice::from_ref(&dep_a))
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
      .track_symbols(&graph, &entry_asset, std::slice::from_ref(&lib_dep))
      .unwrap();
    tracker
      .track_symbols(&graph, &lib_asset, std::slice::from_ref(&a_dep))
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
      lib_final.providing_asset_id, a_asset.id,
      "lib_dep should point to a.js as provider"
    );

    let a_requirements = tracker.requirements_by_dep.get(&a_dep.id).unwrap();
    let a_final = a_requirements[0]
      .final_location
      .as_ref()
      .expect("a_dep requirement should be satisfied");
    assert_eq!(
      a_final.providing_asset_id, a_asset.id,
      "a_dep should point to a.js as provider"
    );
  }

  #[test]
  fn finalize_creates_correct_used_symbols_mapping() {
    let (graph, entry_asset, dep_a, asset_a) = setup_simple_graph();
    let mut tracker = SymbolTracker::default();

    tracker
      .track_symbols(&graph, &entry_asset, std::slice::from_ref(&dep_a))
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

  #[test]
  fn track_symbols_handles_chained_renames() {
    // Test case: chained renames through multiple barrel files
    // index.js imports "finalName" from barrel1
    // barrel1 re-exports "middleName" as "finalName" from barrel2
    // barrel2 re-exports "originalName" as "middleName" from source
    // source exports "originalName"
    //
    // Graph: entry_dep -> index -> barrel1_dep -> barrel1 -> barrel2_dep -> barrel2 -> source_dep -> source

    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    // index.js - imports finalName from barrel1
    let index_asset = make_asset("index.js", vec![]);
    let index_asset_node = graph.add_asset(index_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &index_asset_node);

    // Dependency from index.js to barrel1.js requesting "finalName"
    let barrel1_dep = make_dependency(
      &index_asset,
      "./barrel1.js",
      vec![("$index$import$finalName", "finalName", false)],
    );
    let barrel1_dep_node = graph.add_dependency(barrel1_dep.clone(), false);
    graph.add_edge(&index_asset_node, &barrel1_dep_node);

    // barrel1.js - re-exports middleName as finalName (weak symbol)
    let mangled_reexport_1 = "$barrel1$re_export$finalName";
    let barrel1_asset = make_asset("barrel1.js", vec![(mangled_reexport_1, "finalName", true)]);
    let barrel1_asset_node = graph.add_asset(barrel1_asset.clone(), false);
    graph.add_edge(&barrel1_dep_node, &barrel1_asset_node);

    // Dependency from barrel1.js to barrel2.js requesting "middleName"
    let barrel2_dep = make_dependency(
      &barrel1_asset,
      "./barrel2.js",
      vec![(mangled_reexport_1, "middleName", true)],
    );
    let barrel2_dep_node = graph.add_dependency(barrel2_dep.clone(), false);
    graph.add_edge(&barrel1_asset_node, &barrel2_dep_node);

    // barrel2.js - re-exports originalName as middleName (weak symbol)
    let mangled_reexport_2 = "$barrel2$re_export$middleName";
    let barrel2_asset = make_asset("barrel2.js", vec![(mangled_reexport_2, "middleName", true)]);
    let barrel2_asset_node = graph.add_asset(barrel2_asset.clone(), false);
    graph.add_edge(&barrel2_dep_node, &barrel2_asset_node);

    // Dependency from barrel2.js to source.js requesting "originalName"
    let source_dep = make_dependency(
      &barrel2_asset,
      "./source.js",
      vec![(mangled_reexport_2, "originalName", true)],
    );
    let source_dep_node = graph.add_dependency(source_dep.clone(), false);
    graph.add_edge(&barrel2_asset_node, &source_dep_node);

    // source.js - provides "originalName" (strong symbol)
    let source_asset = make_asset("source.js", vec![("originalName", "originalName", false)]);
    let source_asset_node = graph.add_asset(source_asset.clone(), false);
    graph.add_edge(&source_dep_node, &source_asset_node);

    let mut tracker = SymbolTracker::default();

    // Process in traversal order
    tracker
      .track_symbols(&graph, &index_asset, std::slice::from_ref(&barrel1_dep))
      .unwrap();
    tracker
      .track_symbols(&graph, &barrel1_asset, std::slice::from_ref(&barrel2_dep))
      .unwrap();
    tracker
      .track_symbols(&graph, &barrel2_asset, std::slice::from_ref(&source_dep))
      .unwrap();
    tracker.track_symbols(&graph, &source_asset, &[]).unwrap();

    // Verify all dependencies are satisfied pointing to source_asset
    let barrel1_requirements = tracker.requirements_by_dep.get(&barrel1_dep.id).unwrap();
    let barrel1_final = barrel1_requirements[0]
      .final_location
      .as_ref()
      .expect("barrel1_dep requirement should be satisfied");
    assert_eq!(
      barrel1_final.providing_asset_id, source_asset.id,
      "barrel1_dep should point to source.js as provider"
    );
    assert_eq!(
      barrel1_final.imported_name, "originalName",
      "barrel1_dep should resolve to originalName"
    );

    let barrel2_requirements = tracker.requirements_by_dep.get(&barrel2_dep.id).unwrap();
    let barrel2_final = barrel2_requirements[0]
      .final_location
      .as_ref()
      .expect("barrel2_dep requirement should be satisfied");
    assert_eq!(
      barrel2_final.providing_asset_id, source_asset.id,
      "barrel2_dep should point to source.js as provider"
    );

    let source_requirements = tracker.requirements_by_dep.get(&source_dep.id).unwrap();
    let source_final = source_requirements[0]
      .final_location
      .as_ref()
      .expect("source_dep requirement should be satisfied");
    assert_eq!(
      source_final.providing_asset_id, source_asset.id,
      "source_dep should point to source.js as provider"
    );

    // Finalize should succeed without panic
    let finalized = tracker.finalize();

    // All dependencies should have used symbols
    assert!(
      finalized
        .get_used_symbols_for_dependency(&barrel1_dep.id)
        .is_some(),
      "barrel1_dep should have used symbols"
    );

    // Check that resolved_symbol is correct for barrel1_dep
    let barrel1_used = finalized
      .get_used_symbols_for_dependency(&barrel1_dep.id)
      .unwrap();
    let barrel1_symbol = barrel1_used.values().next().unwrap();
    assert_eq!(
      barrel1_symbol.resolved_symbol, "originalName",
      "barrel1_dep used symbol should resolve to originalName"
    );
  }

  #[test]
  fn track_symbols_handles_star_reexports() {
    // Test case: export * from './foo'
    // index.js imports "foo" from barrel.js
    // barrel.js has `export * from './foo'`
    // foo.js exports "foo"
    //
    // Graph: entry_dep -> index_asset -> barrel_dep -> barrel_asset -> foo_dep -> foo_asset

    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    // index.js - imports foo from barrel
    let index_asset = make_asset("index.js", vec![]);
    let index_asset_node = graph.add_asset(index_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &index_asset_node);

    // Dependency from index.js to barrel.js requesting "foo"
    let barrel_dep = make_dependency(
      &index_asset,
      "./barrel.js",
      vec![("$index$import$foo", "foo", false)],
    );
    let barrel_dep_node = graph.add_dependency(barrel_dep.clone(), false);
    graph.add_edge(&index_asset_node, &barrel_dep_node);

    // barrel.js - has `export * from './foo'` which creates a weak "*" symbol
    // The barrel asset itself has a "*" symbol indicating it re-exports everything
    let barrel_asset = make_asset("barrel.js", vec![("*", "*", true)]);
    let barrel_asset_node = graph.add_asset(barrel_asset.clone(), false);
    graph.add_edge(&barrel_dep_node, &barrel_asset_node);

    // Dependency from barrel.js to foo.js - this is a star re-export dependency
    // It has a "*" -> "*" symbol indicating "export * from './foo'"
    let foo_dep = make_dependency(&barrel_asset, "./foo.js", vec![("*", "*", true)]);
    let foo_dep_node = graph.add_dependency(foo_dep.clone(), false);
    graph.add_edge(&barrel_asset_node, &foo_dep_node);

    // foo.js - provides "foo" (strong symbol)
    let foo_asset = make_asset("foo.js", vec![("foo", "foo", false)]);
    let foo_asset_node = graph.add_asset(foo_asset.clone(), false);
    graph.add_edge(&foo_dep_node, &foo_asset_node);

    let mut tracker = SymbolTracker::default();

    // Process in traversal order
    tracker
      .track_symbols(&graph, &index_asset, std::slice::from_ref(&barrel_dep))
      .unwrap();
    tracker
      .track_symbols(&graph, &barrel_asset, std::slice::from_ref(&foo_dep))
      .unwrap();
    tracker.track_symbols(&graph, &foo_asset, &[]).unwrap();

    // Verify barrel_dep requirement is satisfied
    let barrel_requirements = tracker.requirements_by_dep.get(&barrel_dep.id).unwrap();
    let barrel_final = barrel_requirements[0]
      .final_location
      .as_ref()
      .expect("barrel_dep requirement should be satisfied");
    assert_eq!(
      barrel_final.providing_asset_id, foo_asset.id,
      "barrel_dep should point to foo.js as provider"
    );

    // Verify foo_dep has requirement for "foo" that was propagated from barrel_dep
    let foo_requirements = tracker.requirements_by_dep.get(&foo_dep.id).unwrap();
    assert!(
      foo_requirements.iter().any(|r| r.symbol.exported == "foo"),
      "foo_dep should have a requirement for 'foo' propagated from star re-export"
    );
    let foo_final = foo_requirements
      .iter()
      .find(|r| r.symbol.exported == "foo")
      .unwrap()
      .final_location
      .as_ref()
      .expect("foo_dep requirement should be satisfied");
    assert_eq!(
      foo_final.providing_asset_id, foo_asset.id,
      "foo_dep should point to foo.js as provider"
    );

    // Finalize should succeed without panic
    let finalized = tracker.finalize();

    // Both dependencies should have used symbols
    assert!(
      finalized
        .get_used_symbols_for_dependency(&barrel_dep.id)
        .is_some(),
      "barrel_dep should have used symbols"
    );
    assert!(
      finalized
        .get_used_symbols_for_dependency(&foo_dep.id)
        .is_some(),
      "foo_dep should have used symbols"
    );
  }

  #[test]
  fn track_symbols_handles_renamed_exports() {
    // Test case: export {foo as renamedFoo} from './foo'
    // index.js imports "renamedFoo" from barrel.js
    // barrel.js re-exports "foo" as "renamedFoo" from foo.js
    // foo.js exports "foo"
    //
    // Graph: entry_dep -> index_asset -> barrel_dep -> barrel_asset -> foo_dep -> foo_asset

    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    // index.js - imports renamedFoo from barrel
    let index_asset = make_asset("index.js", vec![]);
    let index_asset_node = graph.add_asset(index_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &index_asset_node);

    // Dependency from index.js to barrel.js requesting "renamedFoo"
    // The local is the mangled import name
    let barrel_dep = make_dependency(
      &index_asset,
      "./barrel.js",
      vec![("$index$import$renamedFoo", "renamedFoo", false)],
    );
    let barrel_dep_node = graph.add_dependency(barrel_dep.clone(), false);
    graph.add_edge(&index_asset_node, &barrel_dep_node);

    // barrel.js - re-exports foo as renamedFoo (weak symbol)
    // The local name here is a mangled re-export identifier that matches between
    // the dependency symbol (barrel->foo) and the asset symbol (barrel)
    let mangled_reexport = "$barrel$re_export$renamedFoo";
    let barrel_asset = make_asset("barrel.js", vec![(mangled_reexport, "renamedFoo", true)]);
    let barrel_asset_node = graph.add_asset(barrel_asset.clone(), false);
    graph.add_edge(&barrel_dep_node, &barrel_asset_node);

    // Dependency from barrel.js to foo.js requesting "foo"
    // The local is the same mangled re-export name
    let foo_dep = make_dependency(
      &barrel_asset,
      "./foo.js",
      vec![(mangled_reexport, "foo", true)],
    );
    let foo_dep_node = graph.add_dependency(foo_dep.clone(), false);
    graph.add_edge(&barrel_asset_node, &foo_dep_node);

    // foo.js - provides "foo" (strong symbol)
    let foo_asset = make_asset("foo.js", vec![("foo", "foo", false)]);
    let foo_asset_node = graph.add_asset(foo_asset.clone(), false);
    graph.add_edge(&foo_dep_node, &foo_asset_node);

    let mut tracker = SymbolTracker::default();

    // Process in traversal order
    tracker
      .track_symbols(&graph, &index_asset, std::slice::from_ref(&barrel_dep))
      .unwrap();
    tracker
      .track_symbols(&graph, &barrel_asset, std::slice::from_ref(&foo_dep))
      .unwrap();
    tracker.track_symbols(&graph, &foo_asset, &[]).unwrap();

    // Verify both dependencies are satisfied pointing to foo_asset
    let barrel_requirements = tracker.requirements_by_dep.get(&barrel_dep.id).unwrap();
    let barrel_final = barrel_requirements[0]
      .final_location
      .as_ref()
      .expect("barrel_dep requirement should be satisfied");
    assert_eq!(
      barrel_final.providing_asset_id, foo_asset.id,
      "barrel_dep should point to foo.js as provider"
    );

    let foo_requirements = tracker.requirements_by_dep.get(&foo_dep.id).unwrap();
    let foo_final = foo_requirements[0]
      .final_location
      .as_ref()
      .expect("foo_dep requirement should be satisfied");
    assert_eq!(
      foo_final.providing_asset_id, foo_asset.id,
      "foo_dep should point to foo.js as provider"
    );

    // Finalize should succeed without panic
    let finalized = tracker.finalize();

    // Both dependencies should have used symbols
    assert!(
      finalized
        .get_used_symbols_for_dependency(&barrel_dep.id)
        .is_some(),
      "barrel_dep should have used symbols"
    );
    assert!(
      finalized
        .get_used_symbols_for_dependency(&foo_dep.id)
        .is_some(),
      "foo_dep should have used symbols"
    );
  }
}
