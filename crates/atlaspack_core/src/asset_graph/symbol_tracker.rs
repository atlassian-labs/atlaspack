use anyhow::anyhow;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::{
  asset_graph::{AssetGraph, AssetGraphNode, DependencyState},
  types::{Asset, AssetId, Dependency, DependencyId, Symbol},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SymbolTracker {
  requirements_by_dep: HashMap<String, Vec<SymbolRequirement>>,
  /// Maps speculation group IDs to the dependency IDs that contain speculative
  /// requirements for that group. This allows efficient lookup and deletion
  /// when one member of the group is satisfied.
  speculation_group_locations: HashMap<String, Vec<String>>,
  /// Maps each asset ID to the symbols it provides — both its own direct exports
  /// and symbols it surfaces through re-exports. The inner map is keyed by exported
  /// symbol name, with the value being the `FinalSymbolLocation` that ultimately
  /// provides that symbol.
  ///
  /// This serves as a cache that enables instant symbol resolution higher up the
  /// tree: when a parent asset re-exports from a child that already has entries in
  /// `provided_symbols`, we can resolve immediately without waiting for bottom-up
  /// propagation.
  provided_symbols: HashMap<AssetId, HashMap<String, FinalSymbolLocation>>,
  /// Tracks which exported symbols are actually used from each asset. Populated
  /// progressively as requirements are satisfied in `satisfy_requirements_from_asset`.
  /// When a requirement on an incoming dependency is satisfied by an asset's export,
  /// that export name is added to the asset's used symbols set.
  used_symbols_by_asset: HashMap<AssetId, HashSet<String>>,
}

/*
* A speculation group is a collection of requirements that might satisfy a * re-export. Since we
* can't know which asset, and thus dependency, will satisfy a symbol requested through a star
* re-export until we process the providing asset, we mark these requirements as speculative and
* group them together. When one member of the group is satisfied, we can remove the other
* speculative requirements since they are mutually exclusive (only one can be correct since they
* come from the same parent symbol request).
*/

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolRequirement {
  pub symbol: Symbol,
  pub final_location: Option<FinalSymbolLocation>,
  /// Links speculative requirements that are mutually exclusive (same symbol from same parent).
  /// Format: `{parent_dep_id}:{symbol_name}`. When one is satisfied, others can be removed.
  /// If Some, this requirement was speculatively forwarded through a star re-export.
  /// If None, this is a direct requirement that must be satisfied.
  pub speculation_group_id: Option<String>,
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
    dep_symbols.iter().any(|s| {
      s.exported == symbol.exported || matches!(classify_symbol_export(s), SymbolExportType::Star)
    })
  })
}

enum SymbolExportType {
  /// A normal export with a specific name (e.g. `export {foo}` or `export {foo as bar}`)
  Named,
  /// A star re-export that forwards all symbols (e.g. `export * from './dep'`)
  Star,
  /// A namespace re-export that creates a namespace object (e.g. `export * as ns from './dep'`)
  Namespace,
}

fn classify_symbol_export(symbol: &Symbol) -> SymbolExportType {
  if symbol.exported != "*" {
    SymbolExportType::Named
  } else if symbol.local == "*" {
    SymbolExportType::Star
  } else {
    SymbolExportType::Namespace
  }
}

/// Represents a symbol request from an incoming dependency.
/// Used for tracking speculation groups when propagating through star re-exports.
struct IncomingSymbolRequest {
  /// The symbol name being requested
  symbol: String,
  /// The dependency ID that originates this request (used for speculation group ID)
  source_dep_id: String,
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

fn get_incoming_dependencies(asset_graph: &AssetGraph, asset_node_id: usize) -> Vec<&Dependency> {
  let incoming_nodes = asset_graph.get_incoming_neighbors(&asset_node_id);
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
/// - The dependency to foo.js has: local=`$assetId$re_export$renamedFoo`, exported='foo'
/// - The barrel asset has: local=`$assetId$re_export$renamedFoo`, exported='renamedFoo'
///
/// So we match on the mangled local name to find the exported name.
fn find_exported_name_for_local(asset: &Asset, mangled_local: &str) -> Option<String> {
  asset
    .symbols
    .as_ref()?
    .iter()
    .find_map(|sym| (sym.local == mangled_local).then(|| sym.exported.clone()))
}

impl SymbolTracker {
  /// Checks whether a dependency has any symbols requested from it.
  ///
  /// This replaces the `get_requested_symbols().is_some_and(|s| !s.is_empty())`
  /// check that `handle_path_result` uses to decide whether to defer a dependency.
  pub fn has_requested_symbols(&self, dependency_id: &DependencyId) -> bool {
    self
      .requirements_by_dep
      .get(dependency_id)
      .is_some_and(|reqs| !reqs.is_empty())
  }

  /// Tracks symbols for an asset and propagates requested symbols to its outgoing
  /// dependencies, returning any dependencies that need to be un-deferred.
  ///
  /// This combines the work of `track_symbols` (registering requirements and
  /// satisfying them) with the propagation logic from `propagate_requested_symbols`
  /// (forwarding symbol requests to outgoing dependencies and signaling un-deferral).
  pub fn track_symbols(
    &mut self,
    asset_graph: &AssetGraph,
    asset: &Arc<Asset>,
    dependencies: &[Dependency],
  ) -> anyhow::Result<Vec<DependencyId>> {
    let incoming_requested_symbols = self.get_incoming_requested_symbols(asset_graph, asset);

    for dep in dependencies {
      self.track_dependency_symbols(asset, &incoming_requested_symbols, dep)?;
    }

    self.satisfy_requirements_from_asset(asset_graph, asset)?;

    // If this asset has incoming dependencies without symbol tracking (e.g. an HTML
    // script tag, or the root entry dependency), add all of the asset's own
    // exported symbols to usedSymbols. This matches the JS propagateSymbols behavior:
    // - Root dependency (sourceAssetId == null): sets isEntry = true
    // - Regular dep with cleared symbols (sourceAssetId != null): sets addAll = true
    // Both cases result in adding all asset symbols to usedSymbols.
    if let Some(asset_node_id) = asset_graph.get_node_id_by_content_key(asset.id.as_str()) {
      let incoming_deps = get_incoming_dependencies(asset_graph, *asset_node_id);
      let has_non_symbol_tracking_incoming = incoming_deps.iter().any(|dep| dep.symbols.is_none());
      if (has_non_symbol_tracking_incoming || incoming_deps.is_empty())
        && let Some(asset_symbols) = &asset.symbols
        && !asset_symbols.is_empty()
      {
        let used = self
          .used_symbols_by_asset
          .entry(asset.id.clone())
          .or_default();
        for sym in asset_symbols {
          used.insert(sym.exported.clone());
        }
      }
    }

    // Returns a Vec of the dependencies that now need to be processed
    self.propagate_to_outgoing_dependencies(asset_graph, asset, dependencies)
  }

  fn track_dependency_symbols(
    &mut self,
    asset: &Arc<Asset>,
    incoming_requested_symbols: &[IncomingSymbolRequest],
    dep: &Dependency,
  ) -> anyhow::Result<()> {
    let Some(symbols) = &dep.symbols else {
      return Ok(());
    };

    // Check if this dependency is a star re-export
    let mut is_star_dep = false;

    for symbol in symbols {
      // Skip star re-export markers (exported="*", local="*") - these are handled
      // separately below via speculative forwarding.
      // But do NOT skip namespace re-exports (exported="*", local="ns") - these are
      // concrete requirements that need to be satisfied by the target asset's namespace.
      match classify_symbol_export(symbol) {
        SymbolExportType::Star => {
          // Star re-export: skip for now, handled separately below
          is_star_dep = true;
          continue;
        }
        SymbolExportType::Namespace => {
          // Namespace re-export: register a "*" requirement on this dependency.
          // This will be satisfied when the target asset is processed.
          self.add_required_symbol(
            &dep.id,
            SymbolRequirement {
              symbol: symbol.clone(),
              final_location: None,
              speculation_group_id: None,
            },
          );
          continue;
        }
        SymbolExportType::Named => {
          self.add_required_symbol(
            &dep.id,
            SymbolRequirement {
              symbol: symbol.clone(),
              final_location: None,
              speculation_group_id: None,
            },
          );
        }
      }
    }

    // For star re-export dependencies, propagate all symbols requested from
    // this asset down to the star dependency. This is how `export * from './dep'`
    // forwards symbol requests to the target module.
    if !is_star_dep {
      return Ok(());
    }

    for request in incoming_requested_symbols {
      // Don't add duplicate requirements
      if symbols.iter().any(|s| s.exported == request.symbol) {
        continue;
      }
      // Don't forward symbols through star deps if the current asset already
      // provides them as a named export (e.g. via a namespace re-export like
      // `export * as ns from './dep'` which creates an "ns" export on the asset).
      // These symbols are handled directly, not via star forwarding.
      if asset
        .symbols
        .as_ref()
        .is_some_and(|syms| syms.iter().any(|s| s.exported == request.symbol))
      {
        continue;
      }
      // Create a forwarded requirement for this symbol
      // Mark as speculative since this star dep might not provide this symbol
      // (another star dep sibling might provide it instead)
      // The speculation_group_id links all speculative requirements for the same
      // symbol from the same parent dependency, so we can clean up siblings when
      // one is satisfied.
      let speculation_group_id = format!("{}:{}", request.source_dep_id, request.symbol);

      // Register this dependency as containing a member of this speculation group
      self
        .speculation_group_locations
        .entry(speculation_group_id.clone())
        .or_default()
        .push(dep.id.clone());

      self.add_required_symbol(
        &dep.id,
        SymbolRequirement {
          symbol: Symbol {
            exported: request.symbol.clone(),
            local: "*".to_string(), // Mark as coming from star re-export
            is_weak: true,
            ..Default::default()
          },
          final_location: None,
          speculation_group_id: Some(speculation_group_id),
        },
      );
    }
    Ok(())
  }

  /// Satisfies outstanding symbol requirements using the symbols provided by this asset.
  ///
  /// For each incoming dependency, checks if any of the asset's exported symbols
  /// match pending requirements, and propagates the resolution upward through
  /// the dependency chain. Also handles namespace re-export satisfaction.
  fn satisfy_requirements_from_asset(
    &mut self,
    asset_graph: &AssetGraph,
    asset: &Arc<Asset>,
  ) -> anyhow::Result<()> {
    let Some(asset_node_id) = asset_graph.get_node_id_by_content_key(asset.id.as_str()) else {
      return Err(anyhow!("Unable to get node ID for asset ID {}", asset.id));
    };

    let incoming_nodes = asset_graph.get_incoming_neighbors(asset_node_id);
    let incoming_deps: Vec<&Dependency> = incoming_nodes
      .iter()
      .filter_map(|node| match node {
        AssetGraphNode::Dependency(dep) => Some(dep.as_ref()),
        _ => None,
      })
      .collect();

    // Check if any incoming dependency has a namespace re-export requirement ("*").
    // For `export * as ns from './dep'`, the dependency to dep has exported="*", local="ns".
    // When we process dep, we satisfy the "*" requirement with dep's own namespace.
    for incoming_dep in &incoming_deps {
      if let Some(dep_symbols) = &incoming_dep.symbols {
        for dep_sym in dep_symbols {
          let SymbolExportType::Namespace = classify_symbol_export(dep_sym) else {
            continue;
          };

          // This asset's entire namespace satisfies the "*" requirement
          let final_location = FinalSymbolLocation {
            local_name: "*".to_string(),
            imported_name: "*".to_string(),
            providing_asset_id: asset.id.clone(),
          };

          self.handle_answer_propagation(asset_graph, &final_location, incoming_dep)?;
        }
      }
    }

    if let Some(asset_symbols) = &asset.symbols {
      for sym in asset_symbols {
        // Skip star symbols on the asset - these are markers for "export *" patterns,
        // not actual symbols that can satisfy requirements
        if sym.exported == "*" {
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

        // Register this asset's own strong symbols in the provided_symbols cache
        self
          .provided_symbols
          .entry(asset.id.clone())
          .or_default()
          .insert(sym.exported.clone(), final_location.clone());

        for incoming_dep in &incoming_deps {
          // If the incoming dependency does not ask for this symbol, skip it
          if !dep_has_symbol(incoming_dep, sym) {
            continue;
          }

          self.handle_answer_propagation(asset_graph, &final_location, incoming_dep)?;
        }
      }
    }

    Ok(())
  }

  fn handle_answer_propagation(
    &mut self,
    asset_graph: &AssetGraph,
    located_symbol: &FinalSymbolLocation,
    dep: &Dependency,
  ) -> anyhow::Result<()> {
    // Work queue contains (dependency, symbol_name_to_match, chain_index)
    // chain_index points into `pending_chain` — the list of (asset_id, symbol)
    // entries accumulated as we propagate upward. When we reach a consumer (not
    // a re-exporter), all entries in the chain are confirmed as used.
    let mut work_queue: Vec<(&Dependency, String, usize)> =
      vec![(dep, located_symbol.imported_name.clone(), 0)];

    // Reusable buffer for matched requirements to avoid repeated allocations
    let mut matched_locals: Vec<String> = Vec::new();

    // Track speculation groups that were satisfied so we can clean up siblings
    let mut satisfied_speculation_groups: Vec<String> = Vec::new();

    // Pending chain of (asset_id, symbol_name) entries. When we reach a consumer
    // (not a re-exporter), we confirm all entries from the chain_start_index onward.
    let mut pending_chain: Vec<(AssetId, String)> = Vec::new();

    // Confirmed used symbols to apply after the work queue is drained.
    let mut confirmed_used: Vec<(AssetId, String)> = Vec::new();

    // Always start with the providing asset's symbol as the first chain entry
    pending_chain.push((
      located_symbol.providing_asset_id.clone(),
      located_symbol.imported_name.clone(),
    ));

    while let Some((dep, symbol_name_to_match, chain_start)) = work_queue.pop() {
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

          // If the requirement is already satisfied, check whether it's the same
          // location (idempotent re-processing, e.g. from replicate_existing_edges
          // in diamond patterns) or a genuine conflict.
          if let Some(ref existing) = required.final_location {
            if existing == located_symbol {
              // Same location — skip (idempotent)
              continue;
            }
            return Err(anyhow!(
              "Required symbol {} for dep [{}] has already been located at {:?}, \
               but a new location was found: {:?}",
              required.symbol.exported,
              dep.id,
              existing,
              located_symbol
            ));
          }

          required.final_location = Some(located_symbol.clone());
          // Store the local name for propagation lookup
          matched_locals.push(required.symbol.local.clone());

          // If this was a speculative requirement, track its group for cleanup
          if let Some(ref group_id) = required.speculation_group_id {
            satisfied_speculation_groups.push(group_id.clone());
          }
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
          get_incoming_dependencies(asset_graph, parent_asset_node_id);

        // Determine the symbol name as seen from the parent asset's perspective
        let symbol_for_parent = if symbol_local == "*" {
          // Star re-export: symbol name passes through unchanged
          symbol_name_to_match.clone()
        } else if let Some(AssetGraphNode::Asset(asset)) = parent_asset_node {
          find_exported_name_for_local(asset, symbol_local)
            .unwrap_or_else(|| symbol_name_to_match.clone())
        } else {
          symbol_name_to_match.clone()
        };

        // Register the resolved symbol in the parent asset's provided_symbols cache.
        // The parent asset now "provides" this symbol through its re-export, even
        // though the final location points to a downstream asset.
        if let Some(AssetGraphNode::Asset(parent_asset)) = parent_asset_node {
          self
            .provided_symbols
            .entry(parent_asset.id.clone())
            .or_default()
            .insert(symbol_for_parent.clone(), located_symbol.clone());
        }

        // Check if the parent asset is a re-exporter (weak symbol or star export).
        // If so, add it to the pending chain. If not, the parent is a consumer
        // and demand is confirmed — all pending chain entries become confirmed used.
        let parent_is_reexport =
          if let Some(AssetGraphNode::Asset(parent_asset)) = parent_asset_node {
            parent_asset.symbols.as_ref().is_some_and(|syms| {
              syms.iter().any(|s| {
                (s.exported == symbol_for_parent && s.is_weak)
                  || matches!(classify_symbol_export(s), SymbolExportType::Star)
              })
            })
          } else {
            false
          };

        let next_chain_start = if parent_is_reexport {
          // Add the re-exporter to the pending chain
          if let Some(AssetGraphNode::Asset(parent_asset)) = parent_asset_node {
            pending_chain.push((parent_asset.id.clone(), symbol_for_parent.clone()));
          }
          chain_start
        } else {
          // Parent is a consumer — confirm all pending chain entries
          for entry in &pending_chain[chain_start..] {
            confirmed_used.push(entry.clone());
          }
          // Future items start a new chain
          pending_chain.len()
        };

        for parent_dep in parent_asset_dependencies {
          work_queue.push((parent_dep, symbol_for_parent.clone(), next_chain_start));
        }
      }
    }

    // Apply confirmed used symbols
    for (asset_id, symbol_name) in confirmed_used {
      self
        .used_symbols_by_asset
        .entry(asset_id)
        .or_default()
        .insert(symbol_name);
    }

    // Clean up sibling speculative requirements that are now unnecessary
    // (another member of the same speculation group was satisfied)
    self.remove_unsatisfied_speculation_siblings(&satisfied_speculation_groups, &dep.id);

    Ok(())
  }

  /// Removes unsatisfied speculative requirements that belong to the given speculation groups.
  /// This is called after a speculative requirement is satisfied - since only one member
  /// of each group needs to be satisfied, the others can be removed.
  ///
  /// This also recursively cleans up any speculation groups that were created from the
  /// removed requirements (for diamond patterns where multiple paths converge).
  fn remove_unsatisfied_speculation_siblings(
    &mut self,
    satisfied_groups: &[String],
    satisfied_dep_id: &str,
  ) {
    let mut groups_to_process: Vec<String> = satisfied_groups.to_vec();
    let mut processed_groups: HashSet<String> = HashSet::new();

    while let Some(group_id) = groups_to_process.pop() {
      // Skip if already processed (avoid infinite loops)
      if !processed_groups.insert(group_id.clone()) {
        continue;
      }

      // Get the dependency IDs that contain members of this speculation group
      let Some(dep_ids) = self.speculation_group_locations.remove(&group_id) else {
        continue;
      };

      // Remove unsatisfied speculative requirements from sibling dependencies
      for dep_id in dep_ids {
        // Skip the dependency that was just satisfied (only for top-level groups)
        if dep_id == satisfied_dep_id && satisfied_groups.contains(&group_id) {
          continue;
        }

        if let Some(requirements) = self.requirements_by_dep.get_mut(&dep_id) {
          requirements.retain(|req| {
            // Keep if not part of this speculation group
            if req.speculation_group_id.as_ref() != Some(&group_id) {
              return true;
            }
            // Keep if already satisfied — removing a satisfied requirement
            // would break the resolution chain in multi-level star re-exports
            req.final_location.is_some()
          });
        }

        // Find which symbols were removed from this dep. For each removed symbol,
        // any downstream speculation groups sourced from this dep for that symbol
        // are now orphaned and need cleanup.
        // Groups are keyed as "{dep_id}:{symbol}", so we can match precisely.
        let remaining_symbols: Vec<String> = self
          .requirements_by_dep
          .get(&dep_id)
          .map(|reqs| reqs.iter().map(|r| r.symbol.exported.clone()).collect())
          .unwrap_or_default();

        let orphaned_groups: Vec<String> = self
          .speculation_group_locations
          .keys()
          .filter(|g| {
            // Match groups sourced from this dep
            // Only orphan if this specific symbol is no longer required on the dep
            g.strip_prefix(&format!("{dep_id}:"))
              .is_some_and(|symbol| !remaining_symbols.contains(&symbol.to_string()))
          })
          .cloned()
          .collect();

        for orphaned_group in orphaned_groups {
          if !processed_groups.contains(&orphaned_group)
            && !groups_to_process.contains(&orphaned_group)
          {
            groups_to_process.push(orphaned_group);
          }
        }
      }
    }
  }

  /// Checks each outgoing dependency of the asset to determine if any need
  /// to be un-deferred. A dependency needs un-deferral when it has no outgoing
  /// edges in the graph (hasn't been resolved yet) AND either:
  /// 1. It has requirements registered on it (symbols are being requested), OR
  /// 2. It is in the `New` state (must always be resolved to determine side effects)
  ///
  /// The second condition matches the behavior of the old `propagate_requested_symbols`
  /// function, which always un-deferred new dependencies regardless of whether they
  /// had requested symbols. This is important for dependencies without symbol tracking
  /// (e.g. `import './polyfill'`, CSS imports) that still need to be resolved.
  ///
  /// Dependencies that already have resolved assets don't need un-deferral —
  /// their symbols will be propagated naturally when those assets are processed.
  fn propagate_to_outgoing_dependencies(
    &self,
    asset_graph: &AssetGraph,
    _asset: &Arc<Asset>,
    dependencies: &[Dependency],
  ) -> anyhow::Result<Vec<DependencyId>> {
    let mut undeferred = Vec::new();

    for dep in dependencies {
      let Some(dep_node_id) = asset_graph.get_node_id_by_content_key(dep.id.as_str()) else {
        continue;
      };

      let has_resolved_asset = !asset_graph.get_outgoing_neighbors(dep_node_id).is_empty();
      if has_resolved_asset {
        continue;
      }

      let has_symbols = self.has_requested_symbols(&dep.id);
      let is_new = *asset_graph.get_dependency_state(dep_node_id) == DependencyState::New;

      if has_symbols || is_new {
        undeferred.push(dep.id.clone());
      }
    }

    Ok(undeferred)
  }

  /// Returns the symbols currently known to be provided by an asset during
  /// graph construction. This can be used for early resolution — if a symbol
  /// is already in this cache, we know where it resolves without needing to
  /// wait for bottom-up propagation.
  pub fn get_provided_symbols_for_asset(
    &self,
    asset_id: &AssetId,
  ) -> Option<&HashMap<String, FinalSymbolLocation>> {
    self.provided_symbols.get(asset_id)
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
  ///
  /// This is used to propagate symbol requests through star re-exports.
  fn get_incoming_requested_symbols(
    &self,
    asset_graph: &AssetGraph,
    asset: &Arc<Asset>,
  ) -> Vec<IncomingSymbolRequest> {
    let Some(asset_node_id) = asset_graph.get_node_id_by_content_key(asset.id.as_str()) else {
      return Vec::new();
    };

    let incoming_deps = get_incoming_dependencies(asset_graph, *asset_node_id);
    let mut seen: HashSet<String> = HashSet::new();
    let mut requested_symbols: Vec<IncomingSymbolRequest> = Vec::new();

    for dep in incoming_deps {
      // Look at requirements that have been registered on this dependency
      if let Some(requirements) = self.requirements_by_dep.get(&dep.id) {
        for req in requirements {
          // Skip if already satisfied or a star symbol
          if req.final_location.is_some() || req.symbol.exported == "*" {
            continue;
          }
          if seen.insert(req.symbol.exported.clone()) {
            requested_symbols.push(IncomingSymbolRequest {
              symbol: req.symbol.exported.clone(),
              source_dep_id: dep.id.clone(),
            });
          }
        }
      }

      // Also look at symbols declared on the dependency itself
      // This handles the case where the dependency hasn't been processed yet
      // but its symbols are already known
      if let Some(symbols) = &dep.symbols {
        for sym in symbols {
          if sym.exported == "*" {
            continue;
          }
          if seen.insert(sym.exported.clone()) {
            requested_symbols.push(IncomingSymbolRequest {
              symbol: sym.exported.clone(),
              source_dep_id: dep.id.clone(),
            });
          }
        }
      }
    }

    requested_symbols
  }

  /// Finalizes the symbol tracker, returning a read-only version of the
  /// [`FinalizedSymbolTracker`] that can be used for serialization and lookup of
  /// symbol locations. This method also validates that there are no outstanding
  /// symbol requirements that have not been satisfied, and that there are no
  /// conflicts.
  #[tracing::instrument(name = "finalize_symbol_tracker", skip_all)]
  pub fn finalize(self) -> FinalizedSymbolTracker {
    // Any speculation groups still in the map were never satisfied
    // (satisfied groups are removed during cleanup)
    assert!(
      self.speculation_group_locations.is_empty(),
      "The following speculation groups were not satisfied: {:?}",
      self.speculation_group_locations.keys()
    );

    let mut requirements_by_dep: HashMap<DependencyId, DependencyUsedSymbols> = HashMap::new();

    for (dep_id, requirements) in self.requirements_by_dep {
      let mut used_symbols = HashMap::new();

      for requirement in requirements {
        // Skip star re-export markers (exported="*", local="*") in finalization -
        // these are forwarding markers, not actual symbols that need to be satisfied.
        // But keep namespace re-export requirements (exported="*", local!="*") -
        // these are real requirements that should have been satisfied.
        if matches!(
          classify_symbol_export(&requirement.symbol),
          SymbolExportType::Star
        ) {
          continue;
        }

        let Some(final_location) = requirement.final_location else {
          // Unsatisfied requirements are expected for unused re-exports.
          // For example, if barrel.js re-exports `bar` from lib2.js but nobody
          // imports `bar`, the requirement stays unsatisfied. This is fine —
          // the dependency will be marked as `excluded` during serialization
          // when the resolved asset has `side_effects: false`.
          //
          // TODO: Detect genuine "missing export" errors (e.g. `import {foo} from './lib'`
          // where lib.js doesn't export `foo`). This requires distinguishing between
          // requirements that were demanded by an upstream consumer vs those eagerly
          // registered by `track_dependency_symbols`.
          continue;
        };

        // Only include resolved requirements in usedSymbolsUp if the providing
        // asset's symbol was actually demanded (appears in used_symbols_by_asset).
        // Requirements that were eagerly registered but never demanded from
        // upstream should not appear in usedSymbolsUp — they are exclude candidates.
        let is_demanded = self
          .used_symbols_by_asset
          .get(&final_location.providing_asset_id)
          .is_some_and(|syms| syms.contains(&final_location.imported_name));

        if !is_demanded {
          continue;
        }

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
      provided_symbols: self.provided_symbols,
      used_symbols_by_asset: self.used_symbols_by_asset,
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
  /// Maps each asset ID to the symbols it provides — both its own direct exports
  /// and symbols it surfaces through re-exports. The inner map is keyed by exported
  /// symbol name, with the value being the `FinalSymbolLocation` that ultimately
  /// provides that symbol.
  provided_symbols: HashMap<AssetId, HashMap<String, FinalSymbolLocation>>,
  /// The set of exported symbols actually used from each asset. Computed
  /// progressively during graph construction as requirements are satisfied.
  used_symbols_by_asset: HashMap<AssetId, HashSet<String>>,
}

impl FinalizedSymbolTracker {
  pub fn get_used_symbols_for_dependency(
    &self,
    dep_id: &DependencyId,
  ) -> Option<&DependencyUsedSymbols> {
    self.requirements_by_dep.get(dep_id)
  }

  /// Returns the symbols provided by an asset, including both its own direct
  /// exports and symbols surfaced through re-exports. The returned map is keyed
  /// by the exported symbol name as seen from this asset's perspective, with the
  /// value being the `FinalSymbolLocation` that ultimately provides the symbol.
  pub fn get_provided_symbols_for_asset(
    &self,
    asset_id: &AssetId,
  ) -> Option<&HashMap<String, FinalSymbolLocation>> {
    self.provided_symbols.get(asset_id)
  }

  /// Returns the set of exported symbols that are actually used from an asset.
  /// This is the Rust equivalent of `assetNode.usedSymbols` in the JS
  /// `propagateSymbols` implementation.
  pub fn get_used_symbols_for_asset(&self, asset_id: &AssetId) -> Option<&HashSet<String>> {
    self.used_symbols_by_asset.get(asset_id)
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
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

  /// Holds the results of a symbol tracker test setup, providing lookup by specifier/file_path.
  struct SymbolTrackerTestCtx {
    tracker: SymbolTracker,
    /// Maps specifier -> Dependency
    deps: HashMap<String, Dependency>,
    /// Maps file_path -> Arc<Asset>
    assets: HashMap<String, Arc<Asset>>,
  }

  impl SymbolTrackerTestCtx {
    /// Assert that a dependency (by specifier) has been resolved to a particular asset (by file path).
    fn assert_dep_resolved_to(&self, dep_specifier: &str, expected_asset_path: &str) {
      let dep = self
        .deps
        .get(dep_specifier)
        .unwrap_or_else(|| panic!("No dep with specifier '{}'", dep_specifier));
      let expected_asset = self
        .assets
        .get(expected_asset_path)
        .unwrap_or_else(|| panic!("No asset with path '{}'", expected_asset_path));

      let requirements = self
        .tracker
        .requirements_by_dep
        .get(&dep.id)
        .unwrap_or_else(|| panic!("No requirements for dep '{}'", dep_specifier));

      let satisfied = requirements.iter().find(|r| {
        r.final_location
          .as_ref()
          .map(|loc| loc.providing_asset_id == expected_asset.id)
          .unwrap_or(false)
      });

      assert!(
        satisfied.is_some(),
        "Expected dep '{}' to be resolved to '{}', but it was not.\nRequirements: {:?}",
        dep_specifier,
        expected_asset_path,
        requirements
      );
    }

    /// Assert that a dependency (by specifier) has a specific symbol resolved to a specific asset,
    /// with a specific resolved symbol name.
    fn assert_symbol_resolved(
      &self,
      dep_specifier: &str,
      symbol_name: &str,
      expected_asset_path: &str,
      expected_resolved_name: &str,
    ) {
      let dep = self
        .deps
        .get(dep_specifier)
        .unwrap_or_else(|| panic!("No dep with specifier '{}'", dep_specifier));
      let expected_asset = self
        .assets
        .get(expected_asset_path)
        .unwrap_or_else(|| panic!("No asset with path '{}'", expected_asset_path));

      let requirements = self
        .tracker
        .requirements_by_dep
        .get(&dep.id)
        .unwrap_or_else(|| panic!("No requirements for dep '{}'", dep_specifier));

      let req = requirements
        .iter()
        .find(|r| r.symbol.exported == symbol_name)
        .unwrap_or_else(|| {
          panic!(
            "No requirement for symbol '{}' on dep '{}'",
            symbol_name, dep_specifier
          )
        });

      let loc = req.final_location.as_ref().unwrap_or_else(|| {
        panic!(
          "Symbol '{}' on dep '{}' was not resolved",
          symbol_name, dep_specifier
        )
      });

      assert_eq!(
        loc.providing_asset_id, expected_asset.id,
        "Symbol '{}' on dep '{}' should be provided by '{}', but was provided by asset '{}'",
        symbol_name, dep_specifier, expected_asset_path, loc.providing_asset_id
      );
      assert_eq!(
        loc.imported_name, expected_resolved_name,
        "Symbol '{}' on dep '{}' should resolve to '{}', but resolved to '{}'",
        symbol_name, dep_specifier, expected_resolved_name, loc.imported_name
      );
    }

    /// Assert that a dependency (by specifier) has an unsatisfied requirement for a symbol.
    fn assert_dep_unsatisfied(&self, dep_specifier: &str, symbol_name: &str) {
      let dep = self
        .deps
        .get(dep_specifier)
        .unwrap_or_else(|| panic!("No dep with specifier '{}'", dep_specifier));
      let requirements = self
        .tracker
        .requirements_by_dep
        .get(&dep.id)
        .unwrap_or_else(|| panic!("No requirements for dep '{}'", dep_specifier));

      let is_unsatisfied = requirements
        .iter()
        .find(|r| r.symbol.exported == symbol_name)
        .is_some_and(|req| req.final_location.is_none());
      assert!(
        is_unsatisfied,
        "Expected symbol '{}' on dep '{}' to be unsatisfied",
        symbol_name, dep_specifier
      );
    }

    /// Assert that a dependency (by specifier) has NO requirement for a given symbol
    /// (e.g., it was cleaned up by speculation).
    fn assert_dep_has_no_requirement(&self, dep_specifier: &str, symbol_name: &str) {
      let dep = self
        .deps
        .get(dep_specifier)
        .unwrap_or_else(|| panic!("No dep with specifier '{}'", dep_specifier));
      let requirements = self.tracker.requirements_by_dep.get(&dep.id);

      let has_symbol = requirements
        .map(|reqs| reqs.iter().any(|r| r.symbol.exported == symbol_name))
        .unwrap_or(false);

      assert!(
        !has_symbol,
        "Expected dep '{}' to have no requirement for '{}', but it does",
        dep_specifier, symbol_name
      );
    }

    /// Assert that an asset (by file path) has exactly the expected set of used symbols.
    fn assert_asset_used_symbols(&self, asset_path: &str, expected: &[&str]) {
      let asset = self
        .assets
        .get(asset_path)
        .unwrap_or_else(|| panic!("No asset with path '{}'", asset_path));

      let actual = self
        .tracker
        .used_symbols_by_asset
        .get(&asset.id)
        .cloned()
        .unwrap_or_default();

      let expected_set: HashSet<String> = expected.iter().map(|s| s.to_string()).collect();

      assert_eq!(
        actual, expected_set,
        "Asset '{}' used symbols mismatch.\n  Expected: {:?}\n  Actual: {:?}",
        asset_path, expected_set, actual
      );
    }

    /// Assert that an asset (by file path) provides a symbol with the expected
    /// final location (providing asset path and resolved symbol name).
    fn assert_asset_provides(
      &self,
      asset_path: &str,
      symbol_name: &str,
      expected_providing_asset_path: &str,
      expected_resolved_name: &str,
    ) {
      let asset = self
        .assets
        .get(asset_path)
        .unwrap_or_else(|| panic!("No asset with path '{}'", asset_path));
      let expected_providing_asset = self
        .assets
        .get(expected_providing_asset_path)
        .unwrap_or_else(|| panic!("No asset with path '{}'", expected_providing_asset_path));

      let provided = self
        .tracker
        .get_provided_symbols_for_asset(&asset.id)
        .unwrap_or_else(|| panic!("No provided symbols for asset '{}'", asset_path));

      let loc = provided.get(symbol_name).unwrap_or_else(|| {
        panic!(
          "Asset '{}' does not provide symbol '{}'. Available: {:?}",
          asset_path,
          symbol_name,
          provided.keys().collect::<Vec<_>>()
        )
      });

      assert_eq!(
        loc.providing_asset_id, expected_providing_asset.id,
        "Asset '{}' provides '{}' from '{}', but expected from '{}'",
        asset_path, symbol_name, loc.providing_asset_id, expected_providing_asset_path
      );
      assert_eq!(
        loc.imported_name, expected_resolved_name,
        "Asset '{}' provides '{}' with resolved name '{}', but expected '{}'",
        asset_path, symbol_name, loc.imported_name, expected_resolved_name
      );
    }

    /// Assert that finalize succeeds and return the finalized tracker.
    fn assert_finalize_ok(self) -> FinalizedSymbolTracker {
      self.tracker.finalize()
    }

    /// Assert that finalize panics with a message containing the expected string.
    /// Returns the panic message for further assertions.
    fn assert_finalize_panics(self, expected_msg: &str) {
      let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        self.tracker.finalize();
      }));
      match result {
        Ok(_) => panic!(
          "Expected finalize to panic with '{}', but it succeeded",
          expected_msg
        ),
        Err(e) => {
          let msg = e
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| e.downcast_ref::<&str>().copied())
            .unwrap_or("(unknown panic)");
          assert!(
            msg.contains(expected_msg),
            "Expected panic message to contain '{}', but got: '{}'",
            expected_msg,
            msg
          );
        }
      }
    }
  }

  /// Build a test graph and run symbol tracking in a declarative way.
  ///
  /// Syntax:
  /// ```ignore
  /// symbol_tracker_test! {
  ///   entry "index.js" provides [] {
  ///     dep "./barrel.js" imports [("local", "exported", false)]
  ///       from "barrel.js" provides [("*", "*", true)] {
  ///       dep "./foo.js" imports [("*", "*", true)]
  ///         from "foo.js" provides [("foo", "foo", false)] {}
  ///     }
  ///   }
  /// }
  /// ```
  macro_rules! symbol_tracker_test {
    (
      entry $entry_path:literal provides [ $( ($el:expr, $ee:expr, $ew:expr) ),* ] {
        $( dep $spec:literal imports [ $( ($dl:expr, $de:expr, $dw:expr) ),* ]
           from $asset_path:literal provides [ $( ($al:expr, $ae:expr, $aw:expr) ),* ]
           { $($children:tt)* }
        )*
      }
    ) => {{
      let mut graph = AssetGraph::new();
      let mut deps: HashMap<String, Dependency> = HashMap::new();
      let mut assets: HashMap<String, Arc<Asset>> = HashMap::new();
      // Traversal order: (asset, outgoing_deps) pairs
      let mut traversal: Vec<(Arc<Asset>, Vec<Dependency>)> = Vec::new();

      let entry_dep = make_entry_dependency();
      let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

      let entry_asset = make_asset($entry_path, vec![ $( ($el, $ee, $ew), )* ]);
      let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
      graph.add_edge(&entry_dep_node, &entry_asset_node);
      assets.insert($entry_path.to_string(), entry_asset.clone());

      let mut entry_outgoing_deps: Vec<Dependency> = Vec::new();

      $(
        symbol_tracker_test!(@add_dep
          graph, deps, assets, traversal,
          entry_asset, entry_asset_node,
          entry_outgoing_deps,
          $spec, [ $( ($dl, $de, $dw) ),* ],
          $asset_path, [ $( ($al, $ae, $aw) ),* ],
          { $($children)* }
        );
      )*

      traversal.insert(0, (entry_asset.clone(), entry_outgoing_deps));

      let mut tracker = SymbolTracker::default();
      for (asset, outgoing) in &traversal {
        tracker.track_symbols(&graph, asset, outgoing).unwrap();
      }

      SymbolTrackerTestCtx { tracker, deps, assets }
    }};

    // Adds a dependency -> asset pair, then recursively processes children.
    // Common setup (dep creation, asset creation/reuse, traversal registration)
    // is shared via the @setup_dep_asset helper.
    (@add_dep
      $graph:ident, $deps:ident, $assets:ident, $traversal:ident,
      $parent_asset:expr, $parent_asset_node:expr,
      $parent_outgoing:ident,
      $spec:literal, [ $( ($dl:expr, $de:expr, $dw:expr) ),* ],
      $asset_path:literal, [ $( ($al:expr, $ae:expr, $aw:expr) ),* ],
      { $($children:tt)+ }
    ) => {
      symbol_tracker_test!(@setup_dep_asset
        $graph, $deps, $assets, $traversal,
        $parent_asset, $parent_asset_node,
        $parent_outgoing,
        $spec, [ $( ($dl, $de, $dw) ),* ],
        $asset_path, [ $( ($al, $ae, $aw) ),* ]
      );

      let asset_ref = $assets.get($asset_path).unwrap().clone();
      let asset_node = *$graph.get_node_id_by_content_key(asset_ref.id.as_str())
        .expect("Asset should have a node after setup");

      // Only add to traversal if this asset hasn't been added yet (diamond pattern)
      let already_tracked = $traversal.iter().any(|(a, _)| a.id == asset_ref.id);
      let traversal_idx = if !already_tracked {
        let idx = $traversal.len();
        $traversal.push((asset_ref.clone(), Vec::new()));
        Some(idx)
      } else {
        None
      };

      let mut child_outgoing: Vec<Dependency> = Vec::new();
      symbol_tracker_test!(@add_children
        $graph, $deps, $assets, $traversal,
        asset_ref, asset_node,
        child_outgoing,
        $($children)*
      );
      if let Some(idx) = traversal_idx {
        $traversal[idx].1 = child_outgoing;
      }
    };

    // Leaf variant: no children (empty braces), so no need for child_outgoing or asset_node.
    (@add_dep
      $graph:ident, $deps:ident, $assets:ident, $traversal:ident,
      $parent_asset:expr, $parent_asset_node:expr,
      $parent_outgoing:ident,
      $spec:literal, [ $( ($dl:expr, $de:expr, $dw:expr) ),* ],
      $asset_path:literal, [ $( ($al:expr, $ae:expr, $aw:expr) ),* ],
      {}
    ) => {
      symbol_tracker_test!(@setup_dep_asset
        $graph, $deps, $assets, $traversal,
        $parent_asset, $parent_asset_node,
        $parent_outgoing,
        $spec, [ $( ($dl, $de, $dw) ),* ],
        $asset_path, [ $( ($al, $ae, $aw) ),* ]
      );

      let asset_ref = $assets.get($asset_path).unwrap().clone();
      let already_tracked = $traversal.iter().any(|(a, _)| a.id == asset_ref.id);
      if !already_tracked {
        $traversal.push((asset_ref, Vec::new()));
      }
    };

    // Common setup: creates the dependency and asset nodes, wires edges,
    // and inserts into the dep/asset maps.
    (@setup_dep_asset
      $graph:ident, $deps:ident, $assets:ident, $traversal:ident,
      $parent_asset:expr, $parent_asset_node:expr,
      $parent_outgoing:ident,
      $spec:literal, [ $( ($dl:expr, $de:expr, $dw:expr) ),* ],
      $asset_path:literal, [ $( ($al:expr, $ae:expr, $aw:expr) ),* ]
    ) => {
      let dep = make_dependency(&$parent_asset, $spec, vec![ $( ($dl, $de, $dw), )* ]);
      let dep_node = $graph.add_dependency(dep.clone(), false);
      $graph.add_edge(&$parent_asset_node, &dep_node);
      $deps.insert($spec.to_string(), dep.clone());
      $parent_outgoing.push(dep.clone());

      let asset = make_asset($asset_path, vec![ $( ($al, $ae, $aw), )* ]);
      if let Some(existing) = $assets.get($asset_path) {
        let existing_node = *$graph.get_node_id_by_content_key(existing.id.as_str())
          .expect("Existing asset should have a node");
        $graph.add_edge(&dep_node, &existing_node);
      } else {
        let new_node = $graph.add_asset(asset.clone(), false);
        $graph.add_edge(&dep_node, &new_node);
        $assets.insert($asset_path.to_string(), asset.clone());
      }
    };

    // Base case: no more children
    (@add_children
      $graph:ident, $deps:ident, $assets:ident, $traversal:ident,
      $parent_asset:expr, $parent_asset_node:expr,
      $parent_outgoing:ident,
    ) => {};

    // Recursive case: process children
    (@add_children
      $graph:ident, $deps:ident, $assets:ident, $traversal:ident,
      $parent_asset:expr, $parent_asset_node:expr,
      $parent_outgoing:ident,
      dep $spec:literal imports [ $( ($dl:expr, $de:expr, $dw:expr) ),* ]
        from $asset_path:literal provides [ $( ($al:expr, $ae:expr, $aw:expr) ),* ]
        { $($children:tt)* }
      $($rest:tt)*
    ) => {
      symbol_tracker_test!(@add_dep
        $graph, $deps, $assets, $traversal,
        $parent_asset, $parent_asset_node,
        $parent_outgoing,
        $spec, [ $( ($dl, $de, $dw) ),* ],
        $asset_path, [ $( ($al, $ae, $aw) ),* ],
        { $($children)* }
      );
      symbol_tracker_test!(@add_children
        $graph, $deps, $assets, $traversal,
        $parent_asset, $parent_asset_node,
        $parent_outgoing,
        $($rest)*
      );
    };
  }

  #[test]
  fn track_symbols_registers_and_satisfies_requirements() {
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./a.js" imports [("a", "a", false)] from "a.js" provides [("a", "a", false)] {}
      }
    };

    ctx.assert_symbol_resolved("./a.js", "a", "a.js", "a");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_does_not_satisfy_with_weak_symbols() {
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./lib.js" imports [("a", "a", false)] from "lib.js" provides [("a", "a", true)] {}
      }
    };

    ctx.assert_dep_unsatisfied("./lib.js", "a");
  }

  #[test]
  fn track_symbols_propagates_answer_up_dependency_chain() {
    // entry -> lib (weak re-export) -> a.js (strong provider)
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./lib.js" imports [("a", "a", false)] from "lib.js" provides [("a", "a", true)] {
          dep "./a.js" imports [("a", "a", false)] from "a.js" provides [("a", "a", false)] {}
        }
      }
    };

    ctx.assert_symbol_resolved("./lib.js", "a", "a.js", "a");
    ctx.assert_symbol_resolved("./a.js", "a", "a.js", "a");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn finalize_creates_correct_used_symbols_mapping() {
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./a.js" imports [("a", "a", false)] from "a.js" provides [("a", "a", false)] {}
      }
    };

    ctx.assert_symbol_resolved("./a.js", "a", "a.js", "a");

    let dep_id = ctx.deps.get("./a.js").unwrap().id.clone();
    let finalized = ctx.assert_finalize_ok();
    let used_symbols = finalized
      .get_used_symbols_for_dependency(&dep_id)
      .expect("Should have used symbols for dep");

    assert_eq!(used_symbols.len(), 1);
    let used_symbol = used_symbols.values().next().unwrap();
    assert_eq!(used_symbol.symbol.exported, "a");
  }

  #[test]
  fn finalize_allows_unsatisfied_requirements_for_exclusion() {
    // lib.js has a weak symbol only, so the requirement for "a" will not be
    // satisfied (no strong export). This should NOT panic — unsatisfied
    // requirements are exclude candidates (the dependency can be excluded
    // if the resolved asset has side_effects: false).
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./lib.js" imports [("a", "a", false)] from "lib.js" provides [("a", "a", true)] {}
      }
    };

    let dep = ctx.deps.get("./lib.js").unwrap().clone();
    let finalized = ctx.assert_finalize_ok();

    // The dep should have an empty usedSymbolsUp (no resolved symbols)
    let used = finalized.get_used_symbols_for_dependency(&dep.id);
    assert!(
      used.is_none() || used.unwrap().is_empty(),
      "Unsatisfied requirements should produce empty usedSymbolsUp"
    );
  }

  // NOTE: Manual graph construction required — this test calls track_symbols on the
  // same asset twice to verify idempotent re-processing, which the macro cannot express.
  #[test]
  fn track_symbols_idempotent_when_same_asset_processed_twice() {
    // When the same asset is processed twice (e.g. via replicate_existing_edges),
    // the same symbol resolves to the same location — this should be a no-op.
    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let entry_asset = make_asset("entry.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    let dep_a = make_dependency(&entry_asset, "./a.js", vec![("a", "a", false)]);
    let dep_a_node = graph.add_dependency(dep_a.clone(), false);
    graph.add_edge(&entry_asset_node, &dep_a_node);

    let asset_a = make_asset("a.js", vec![("a", "a", false)]);
    let asset_a_node = graph.add_asset(asset_a.clone(), false);
    graph.add_edge(&dep_a_node, &asset_a_node);

    let mut tracker = SymbolTracker::default();
    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a])
      .unwrap();
    tracker.track_symbols(&graph, &asset_a, &[]).unwrap();

    // Processing the same asset again should be idempotent (same location)
    let result = tracker.track_symbols(&graph, &asset_a, &[]);
    assert!(
      result.is_ok(),
      "Re-processing the same asset should be idempotent, but got: {:?}",
      result.unwrap_err()
    );
  }

  // NOTE: Manual graph construction required — this test links two different assets
  // to the same dependency node to test conflict detection, which the macro cannot express.
  #[test]
  fn track_symbols_errors_on_conflicting_symbol_resolution() {
    // When two DIFFERENT assets provide the same symbol to the same dependency,
    // the second resolution should error because the locations differ.
    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let entry_asset = make_asset("entry.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    let dep_a = make_dependency(&entry_asset, "./a.js", vec![("a", "a", false)]);
    let dep_a_node = graph.add_dependency(dep_a.clone(), false);
    graph.add_edge(&entry_asset_node, &dep_a_node);

    // First asset provides "a"
    let asset_a = make_asset("a.js", vec![("a", "a", false)]);
    let asset_a_node = graph.add_asset(asset_a.clone(), false);
    graph.add_edge(&dep_a_node, &asset_a_node);

    // Second, different asset also provides "a" (different asset ID)
    let asset_b = make_asset("b.js", vec![("a", "a", false)]);
    let asset_b_node = graph.add_asset(asset_b.clone(), false);
    graph.add_edge(&dep_a_node, &asset_b_node);

    let mut tracker = SymbolTracker::default();
    tracker
      .track_symbols(&graph, &entry_asset, &[dep_a])
      .unwrap();
    // First asset satisfies the requirement
    tracker.track_symbols(&graph, &asset_a, &[]).unwrap();

    // Second, different asset tries to satisfy the same requirement — conflict!
    let result = tracker.track_symbols(&graph, &asset_b, &[]);
    assert!(
      result.is_err(),
      "Conflicting symbol resolution should error"
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
    // index.js -> barrel1 (finalName) -> barrel2 (middleName) -> source (originalName)
    let m1 = "$barrel1$re_export$finalName";
    let m2 = "$barrel2$re_export$middleName";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel1.js" imports [("$index$import$finalName", "finalName", false)]
          from "barrel1.js" provides [(m1, "finalName", true)] {
          dep "./barrel2.js" imports [(m1, "middleName", true)]
            from "barrel2.js" provides [(m2, "middleName", true)] {
            dep "./source.js" imports [(m2, "originalName", true)]
              from "source.js" provides [("originalName", "originalName", false)] {}
          }
        }
      }
    };

    ctx.assert_symbol_resolved("./barrel1.js", "finalName", "source.js", "originalName");
    ctx.assert_symbol_resolved("./barrel2.js", "middleName", "source.js", "originalName");
    ctx.assert_symbol_resolved("./source.js", "originalName", "source.js", "originalName");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_star_reexports() {
    // barrel.js has `export * from './foo'`, foo.js provides "foo"
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$foo", "foo", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./foo.js" imports [("*", "*", true)] from "foo.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    ctx.assert_symbol_resolved("./barrel.js", "foo", "foo.js", "foo");
    ctx.assert_dep_resolved_to("./foo.js", "foo.js");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_renamed_exports() {
    // barrel.js re-exports foo as renamedFoo via mangled local
    let m = "$barrel$re_export$renamedFoo";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$renamedFoo", "renamedFoo", false)]
          from "barrel.js" provides [(m, "renamedFoo", true)] {
          dep "./foo.js" imports [(m, "foo", true)] from "foo.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    ctx.assert_symbol_resolved("./barrel.js", "renamedFoo", "foo.js", "foo");
    ctx.assert_symbol_resolved("./foo.js", "foo", "foo.js", "foo");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_cleans_up_speculative_siblings_when_one_is_satisfied() {
    // barrel.js has 3 star re-exports, only sub-dep-2 provides "mySymbol"
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$mySymbol", "mySymbol", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./sub-dep-1.js" imports [("*", "*", true)] from "sub-dep-1.js" provides [("other1", "other1", false)] {}
          dep "./sub-dep-2.js" imports [("*", "*", true)] from "sub-dep-2.js" provides [("mySymbol", "mySymbol", false)] {}
          dep "./sub-dep-3.js" imports [("*", "*", true)] from "sub-dep-3.js" provides [("other3", "other3", false)] {}
        }
      }
    };

    // Only sub-dep-2 should have mySymbol; siblings should be cleaned up
    ctx.assert_dep_has_no_requirement("./sub-dep-1.js", "mySymbol");
    ctx.assert_symbol_resolved("./sub-dep-2.js", "mySymbol", "sub-dep-2.js", "mySymbol");
    ctx.assert_dep_has_no_requirement("./sub-dep-3.js", "mySymbol");
    ctx.assert_symbol_resolved("./barrel.js", "mySymbol", "sub-dep-2.js", "mySymbol");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_diamond_pattern_star_reexports() {
    // Diamond: barrel -> left -> shared, barrel -> right -> shared
    // Both paths converge on shared.js which provides "foo"
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$foo", "foo", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./left.js" imports [("*", "*", true)] from "left.js" provides [("*", "*", true)] {
            dep "./left-shared.js" imports [("*", "*", true)] from "shared.js" provides [("foo", "foo", false)] {}
          }
          dep "./right.js" imports [("*", "*", true)] from "right.js" provides [("*", "*", true)] {
            dep "./right-shared.js" imports [("*", "*", true)] from "shared.js" provides [("foo", "foo", false)] {}
          }
        }
      }
    };

    ctx.assert_symbol_resolved("./barrel.js", "foo", "shared.js", "foo");
    ctx.assert_finalize_ok();
  }

  // NOTE: Manual graph construction required — this test calls track_symbols on
  // the shared asset twice to simulate replicate_existing_edges, which the macro
  // cannot express (it processes each asset exactly once).
  #[test]
  fn track_symbols_handles_diamond_pattern_with_replicated_edges() {
    // Simulates what happens when replicate_existing_edges calls track_symbols
    // a second time for the shared asset in a diamond pattern.
    //
    // In a real build: left.js and right.js both resolve to the same shared.js.
    // The first path processes shared.js normally; the second path triggers
    // replicate_existing_edges which calls track_symbols(shared.js) again.
    let mut graph = AssetGraph::new();
    let mut deps: HashMap<String, Dependency> = HashMap::new();
    let mut assets: HashMap<String, Arc<Asset>> = HashMap::new();

    // Build the diamond graph
    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let index = make_asset("index.js", vec![]);
    let index_node = graph.add_asset(index.clone(), false);
    graph.add_edge(&entry_dep_node, &index_node);
    assets.insert("index.js".to_string(), index.clone());

    let dep_barrel = make_dependency(
      &index,
      "./barrel.js",
      vec![("$index$import$foo", "foo", false)],
    );
    let dep_barrel_node = graph.add_dependency(dep_barrel.clone(), false);
    graph.add_edge(&index_node, &dep_barrel_node);
    deps.insert("./barrel.js".to_string(), dep_barrel.clone());

    let barrel = make_asset("barrel.js", vec![("*", "*", true)]);
    let barrel_node = graph.add_asset(barrel.clone(), false);
    graph.add_edge(&dep_barrel_node, &barrel_node);
    assets.insert("barrel.js".to_string(), barrel.clone());

    let dep_left = make_dependency(&barrel, "./left.js", vec![("*", "*", true)]);
    let dep_left_node = graph.add_dependency(dep_left.clone(), false);
    graph.add_edge(&barrel_node, &dep_left_node);
    deps.insert("./left.js".to_string(), dep_left.clone());

    let left = make_asset("left.js", vec![("*", "*", true)]);
    let left_node = graph.add_asset(left.clone(), false);
    graph.add_edge(&dep_left_node, &left_node);
    assets.insert("left.js".to_string(), left.clone());

    let dep_right = make_dependency(&barrel, "./right.js", vec![("*", "*", true)]);
    let dep_right_node = graph.add_dependency(dep_right.clone(), false);
    graph.add_edge(&barrel_node, &dep_right_node);
    deps.insert("./right.js".to_string(), dep_right.clone());

    let right = make_asset("right.js", vec![("*", "*", true)]);
    let right_node = graph.add_asset(right.clone(), false);
    graph.add_edge(&dep_right_node, &right_node);
    assets.insert("right.js".to_string(), right.clone());

    // shared.js is the SAME asset, reached via two different deps
    let shared = make_asset("shared.js", vec![("foo", "foo", false)]);
    let shared_node = graph.add_asset(shared.clone(), false);
    assets.insert("shared.js".to_string(), shared.clone());

    let dep_left_shared = make_dependency(&left, "./left-shared.js", vec![("*", "*", true)]);
    let dep_left_shared_node = graph.add_dependency(dep_left_shared.clone(), false);
    graph.add_edge(&left_node, &dep_left_shared_node);
    graph.add_edge(&dep_left_shared_node, &shared_node);
    deps.insert("./left-shared.js".to_string(), dep_left_shared.clone());

    let dep_right_shared = make_dependency(&right, "./right-shared.js", vec![("*", "*", true)]);
    let dep_right_shared_node = graph.add_dependency(dep_right_shared.clone(), false);
    graph.add_edge(&right_node, &dep_right_shared_node);
    graph.add_edge(&dep_right_shared_node, &shared_node);
    deps.insert("./right-shared.js".to_string(), dep_right_shared.clone());

    // Process in order, simulating the real build
    let mut tracker = SymbolTracker::default();
    tracker
      .track_symbols(&graph, &index, std::slice::from_ref(&dep_barrel))
      .unwrap();
    tracker
      .track_symbols(&graph, &barrel, &[dep_left.clone(), dep_right.clone()])
      .unwrap();
    tracker
      .track_symbols(&graph, &left, std::slice::from_ref(&dep_left_shared))
      .unwrap();
    tracker
      .track_symbols(&graph, &right, std::slice::from_ref(&dep_right_shared))
      .unwrap();
    // First call: shared.js processed via left path
    tracker.track_symbols(&graph, &shared, &[]).unwrap();
    // Second call: replicate_existing_edges triggers track_symbols again for shared.js
    // This simulates what happens when the right path connects to the already-processed shared.js
    tracker.track_symbols(&graph, &shared, &[]).unwrap();

    // Should still resolve correctly
    let ctx = SymbolTrackerTestCtx {
      tracker,
      deps,
      assets,
    };
    ctx.assert_symbol_resolved("./barrel.js", "foo", "shared.js", "foo");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_chained_star_reexports() {
    // index.js imports {foo, bar} from barrel
    // barrel.js: export * from './foo'; export * from './bar'
    // foo.js: export * from './sub-foo'; export function topLevelFoo
    // sub-foo.js: export function foo; export function unusedFoo
    // bar.js: export const bar; export const unusedBar
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$foo", "foo", false), ("$index$import$bar", "bar", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./foo.js" imports [("*", "*", true)]
            from "foo.js" provides [("topLevelFoo", "topLevelFoo", false), ("*", "*", true)] {
            dep "./sub-foo.js" imports [("*", "*", true)]
              from "sub-foo.js" provides [("foo", "foo", false), ("unusedFoo", "unusedFoo", false)] {}
          }
          dep "./bar.js" imports [("*", "*", true)]
            from "bar.js" provides [("bar", "bar", false), ("unusedBar", "unusedBar", false)] {}
        }
      }
    };

    ctx.assert_symbol_resolved("./barrel.js", "foo", "sub-foo.js", "foo");
    ctx.assert_symbol_resolved("./barrel.js", "bar", "bar.js", "bar");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn finalize_panics_when_speculation_group_not_satisfied() {
    // barrel.js star re-exports sub.js, but sub.js doesn't have "missingSymbol"
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$missingSymbol", "missingSymbol", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./sub.js" imports [("*", "*", true)] from "sub.js" provides [("otherSymbol", "otherSymbol", false)] {}
        }
      }
    };

    ctx.assert_finalize_panics("speculation groups were not satisfied");
  }

  // =====================================================================
  // Namespace re-export tests: `export * as ns from './dep'`
  // =====================================================================

  #[test]
  fn track_symbols_handles_basic_namespace_reexport() {
    // barrel.js has `export * as ns from './dep'`
    // index.js imports `ns` from barrel
    //
    // The dependency from barrel to dep has: exported="*", local="ns"
    // barrel.js asset has a symbol: exported="ns", local="ns"
    // This is NOT a star re-export (local != "*"), so it should behave like
    // a named re-export where the whole namespace is the value.
    let m = "$barrel$re_export$ns";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$ns", "ns", false)]
          from "barrel.js" provides [(m, "ns", true)] {
          dep "./dep.js" imports [(m, "*", true)]
            from "dep.js" provides [("foo", "foo", false), ("bar", "bar", false)] {}
        }
      }
    };

    // The namespace import should resolve: barrel.js's "ns" should point to dep.js
    // The dep's exported symbol is "*" meaning the whole namespace
    ctx.assert_symbol_resolved("./barrel.js", "ns", "dep.js", "*");
    ctx.assert_symbol_resolved("./dep.js", "*", "dep.js", "*");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_namespace_reexport_with_other_named_exports() {
    let m = "$barrel$re_export$ns";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$ns", "ns", false), ("$index$import$localFn", "localFn", false)]
          from "barrel.js" provides [(m, "ns", true), ("localFn", "localFn", false)] {
          dep "./dep.js" imports [(m, "*", true)]
            from "dep.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    // "ns" should resolve to dep.js's namespace
    ctx.assert_symbol_resolved("./barrel.js", "ns", "dep.js", "*");
    // "localFn" should resolve to barrel.js's own export
    ctx.assert_symbol_resolved("./barrel.js", "localFn", "barrel.js", "localFn");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_namespace_reexport_alongside_star_reexport() {
    let m = "$barrel$re_export$ns";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$foo", "foo", false), ("$index$import$ns", "ns", false)]
          from "barrel.js" provides [("*", "*", true), (m, "ns", true)] {
          dep "./star-dep.js" imports [("*", "*", true)]
            from "star-dep.js" provides [("foo", "foo", false)] {}
          dep "./ns-dep.js" imports [(m, "*", true)]
            from "ns-dep.js" provides [("bar", "bar", false)] {}
        }
      }
    };

    // "foo" resolves through the star re-export to star-dep.js
    ctx.assert_symbol_resolved("./barrel.js", "foo", "star-dep.js", "foo");
    // "ns" resolves to ns-dep.js's namespace
    ctx.assert_symbol_resolved("./barrel.js", "ns", "ns-dep.js", "*");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_chained_namespace_reexport() {
    let m1 = "$barrel1$re_export$innerNs";
    let m2 = "$barrel2$re_export$deepNs";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel1.js" imports [("$index$import$innerNs", "innerNs", false)]
          from "barrel1.js" provides [(m1, "innerNs", true)] {
          dep "./barrel2.js" imports [(m1, "*", true)]
            from "barrel2.js" provides [(m2, "deepNs", true)] {
            dep "./source.js" imports [(m2, "*", true)]
              from "source.js" provides [("foo", "foo", false)] {}
          }
        }
      }
    };

    // innerNs should resolve to barrel2.js's namespace (since that's the whole module)
    ctx.assert_symbol_resolved("./barrel1.js", "innerNs", "barrel2.js", "*");
    ctx.assert_symbol_resolved("./barrel2.js", "*", "barrel2.js", "*");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn track_symbols_handles_multiple_namespace_reexports_from_same_barrel() {
    let mfoo = "$barrel$re_export$nsFoo";
    let mbar = "$barrel$re_export$nsBar";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$nsFoo", "nsFoo", false), ("$index$import$nsBar", "nsBar", false)]
          from "barrel.js" provides [(mfoo, "nsFoo", true), (mbar, "nsBar", true)] {
          dep "./foo.js" imports [(mfoo, "*", true)]
            from "foo.js" provides [("a", "a", false)] {}
          dep "./bar.js" imports [(mbar, "*", true)]
            from "bar.js" provides [("b", "b", false)] {}
        }
      }
    };

    ctx.assert_symbol_resolved("./barrel.js", "nsFoo", "foo.js", "*");
    ctx.assert_symbol_resolved("./barrel.js", "nsBar", "bar.js", "*");
    ctx.assert_finalize_ok();
  }

  // ============================================================
  // has_requested_symbols unit tests
  //
  // NOTE: Manual construction required — these test SymbolTracker
  // internals directly without an asset graph.
  // ============================================================

  #[test]
  fn has_requested_symbols_returns_false_for_unknown_dep() {
    let tracker = SymbolTracker::default();
    assert!(!tracker.has_requested_symbols(&"nonexistent".to_string()));
  }

  #[test]
  fn has_requested_symbols_returns_false_for_empty_requirements() {
    let mut tracker = SymbolTracker::default();
    tracker
      .requirements_by_dep
      .insert("dep1".to_string(), Vec::new());
    assert!(!tracker.has_requested_symbols(&"dep1".to_string()));
  }

  #[test]
  fn has_requested_symbols_returns_true_when_requirements_exist() {
    let mut tracker = SymbolTracker::default();
    tracker.add_required_symbol(
      "dep1",
      SymbolRequirement {
        symbol: Symbol {
          exported: "foo".to_string(),
          local: "foo".to_string(),
          ..Default::default()
        },
        final_location: None,
        speculation_group_id: None,
      },
    );
    assert!(tracker.has_requested_symbols(&"dep1".to_string()));
  }

  // NOTE: Manual graph construction required — this test intentionally leaves a
  // dependency without a resolved asset to test un-deferral signaling, which the
  // macro cannot express (it always creates an asset for each dependency).
  #[test]
  fn track_symbols_returns_undeferred_for_unresolved_deps() {
    // Build a graph where index.js imports "a" from ./a.js,
    // but ./a.js has NO resolved asset (no outgoing edge from dep node).
    // track_symbols should signal that this dep needs un-deferral.
    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let entry_asset = make_asset("index.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    // Create dep to a.js but do NOT add an asset for it (unresolved)
    let dep_a = make_dependency(&entry_asset, "./a.js", vec![("a", "a", false)]);
    let dep_a_node = graph.add_dependency(dep_a.clone(), false);
    graph.add_edge(&entry_asset_node, &dep_a_node);

    let mut tracker = SymbolTracker::default();
    let undeferred = tracker
      .track_symbols(&graph, &entry_asset, std::slice::from_ref(&dep_a))
      .unwrap();

    assert_eq!(undeferred, vec![dep_a.id()]);
  }

  #[test]
  fn track_symbols_does_not_undefer_resolved_deps() {
    // Build a graph where index.js imports "a" from ./a.js,
    // and ./a.js IS resolved (has an outgoing edge to an asset).
    // track_symbols should NOT signal un-deferral.
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./a.js" imports [("a", "a", false)] from "a.js" provides [("a", "a", false)] {}
      }
    };

    // The macro resolves deps, so un-deferral should not be signaled.
    // Verify by checking that the symbol was resolved (meaning the dep was
    // already connected to a resolved asset, so no un-deferral needed).
    ctx.assert_symbol_resolved("./a.js", "a", "a.js", "a");
  }

  #[test]
  fn track_symbols_does_not_undefer_deps_without_requirements_when_resolved() {
    // A dep with no symbols (e.g., side-effect import `import './setup'`)
    // that already has a resolved asset should NOT be un-deferred.
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./setup.js" imports [] from "setup.js" provides [] {}
      }
    };

    // With no symbols imported, no requirements should exist
    let dep = ctx.deps.get("./setup.js").unwrap();
    assert!(
      !ctx.tracker.has_requested_symbols(&dep.id),
      "Should not have requested symbols for a side-effect import"
    );
  }

  #[test]
  fn track_symbols_undefers_new_deps_without_symbols() {
    // A dependency with no symbols (e.g., `import './polyfill'`) that is in the
    // New state and has no resolved asset must still be un-deferred so it can be
    // resolved and checked for side effects. This matches the behavior of the old
    // `propagate_requested_symbols` function which always un-deferred New deps.
    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let entry_asset = make_asset("index.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    // Create a dependency with NO symbols (like `import './polyfill'`)
    // and do NOT connect it to any asset — it's unresolved
    let polyfill_dep = make_dependency(&entry_asset, "./polyfill.js", vec![]);
    let polyfill_dep_node = graph.add_dependency(polyfill_dep.clone(), false);
    graph.add_edge(&entry_asset_node, &polyfill_dep_node);

    let mut tracker = SymbolTracker::default();
    let undeferred = tracker
      .track_symbols(&graph, &entry_asset, &[polyfill_dep.clone()])
      .unwrap();

    // The dependency should be un-deferred even though it has no symbols,
    // because it's in the New state and needs to be resolved for side effects
    assert_eq!(
      undeferred,
      vec![polyfill_dep.id.clone()],
      "New dependencies without symbols should still be un-deferred"
    );
  }

  #[test]
  fn track_symbols_does_not_undefer_deferred_deps_without_symbols() {
    // A dependency with no symbols that has been explicitly deferred should NOT
    // be un-deferred by the symbol tracker — only New state deps qualify.
    let mut graph = AssetGraph::new();

    let entry_dep = make_entry_dependency();
    let entry_dep_node = graph.add_entry_dependency(entry_dep, false);

    let entry_asset = make_asset("index.js", vec![]);
    let entry_asset_node = graph.add_asset(entry_asset.clone(), false);
    graph.add_edge(&entry_dep_node, &entry_asset_node);

    // Create a dependency with no symbols and mark it as Deferred
    let deferred_dep = make_dependency(&entry_asset, "./deferred.js", vec![]);
    let deferred_dep_node = graph.add_dependency(deferred_dep.clone(), false);
    graph.add_edge(&entry_asset_node, &deferred_dep_node);
    graph.set_dependency_state(&deferred_dep_node, DependencyState::Deferred);

    let mut tracker = SymbolTracker::default();
    let undeferred = tracker
      .track_symbols(&graph, &entry_asset, &[deferred_dep.clone()])
      .unwrap();

    // Deferred deps without symbols should NOT be un-deferred
    assert!(
      undeferred.is_empty(),
      "Deferred dependencies without symbols should not be un-deferred"
    );
  }

  #[test]
  fn provided_symbols_tracks_direct_exports() {
    // a.js exports "a" directly — it should appear in provided_symbols for a.js
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./a.js" imports [("a", "a", false)] from "a.js" provides [("a", "a", false)] {}
      }
    };

    ctx.assert_asset_provides("a.js", "a", "a.js", "a");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn provided_symbols_tracks_reexports_on_barrel() {
    // barrel.js re-exports "a" from a.js (weak symbol on barrel, strong on a.js)
    // barrel.js should have "a" in its provided_symbols, pointing to a.js
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("a", "a", false)]
          from "barrel.js" provides [("a", "a", true)] {
          dep "./a.js" imports [("a", "a", false)]
            from "a.js" provides [("a", "a", false)] {}
        }
      }
    };

    // a.js provides "a" directly
    ctx.assert_asset_provides("a.js", "a", "a.js", "a");
    // barrel.js provides "a" through re-export, pointing to a.js
    ctx.assert_asset_provides("barrel.js", "a", "a.js", "a");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn provided_symbols_tracks_star_reexports() {
    // barrel.js does `export * from './foo.js'` — foo's symbols should appear
    // in barrel's provided_symbols
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("foo", "foo", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./foo.js" imports [("*", "*", true)]
            from "foo.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    ctx.assert_asset_provides("foo.js", "foo", "foo.js", "foo");
    ctx.assert_asset_provides("barrel.js", "foo", "foo.js", "foo");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn provided_symbols_tracks_renamed_reexports() {
    // barrel.js does `export {originalName as renamedName} from './source.js'`
    // barrel.js should provide "renamedName" pointing to source.js's "originalName"
    let m = "$barrel$re_export$renamedName";

    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$index$import$renamedName", "renamedName", false)]
          from "barrel.js" provides [(m, "renamedName", true)] {
          dep "./source.js" imports [(m, "originalName", true)]
            from "source.js" provides [("originalName", "originalName", false)] {}
        }
      }
    };

    ctx.assert_asset_provides("source.js", "originalName", "source.js", "originalName");
    ctx.assert_asset_provides("barrel.js", "renamedName", "source.js", "originalName");
    ctx.assert_finalize_ok();
  }

  #[test]
  fn provided_symbols_available_on_finalized_tracker() {
    // Verify that provided_symbols flows through to FinalizedSymbolTracker
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("a", "a", false)]
          from "barrel.js" provides [("a", "a", true)] {
          dep "./a.js" imports [("a", "a", false)]
            from "a.js" provides [("a", "a", false)] {}
        }
      }
    };

    let barrel_id = ctx.assets.get("barrel.js").unwrap().id.clone();
    let a_id = ctx.assets.get("a.js").unwrap().id.clone();

    let finalized = ctx.assert_finalize_ok();

    // Check a.js provides "a" directly
    let a_provided = finalized
      .get_provided_symbols_for_asset(&a_id)
      .expect("a.js should have provided symbols");
    let a_loc = a_provided.get("a").expect("a.js should provide 'a'");
    assert_eq!(a_loc.providing_asset_id, a_id);
    assert_eq!(a_loc.imported_name, "a");

    // Check barrel.js provides "a" through re-export
    let barrel_provided = finalized
      .get_provided_symbols_for_asset(&barrel_id)
      .expect("barrel.js should have provided symbols");
    let barrel_loc = barrel_provided
      .get("a")
      .expect("barrel.js should provide 'a'");
    assert_eq!(barrel_loc.providing_asset_id, a_id);
    assert_eq!(barrel_loc.imported_name, "a");
  }

  // ---- used_symbols_by_asset tests ----

  #[test]
  fn used_symbols_tracks_direct_import() {
    // index.js imports {foo} from ./provider.js
    // provider.js exports foo (strong) and bar (strong)
    // Only foo should be in provider's usedSymbols
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./provider.js" imports [("foo", "foo", false)]
          from "provider.js" provides [("foo", "foo", false), ("bar", "bar", false)] {}
      }
    };

    ctx.assert_asset_used_symbols("provider.js", &["foo"]);
    // index.js is an entry asset with no exports — usedSymbols is empty
    ctx.assert_asset_used_symbols("index.js", &[]);
  }

  #[test]
  fn used_symbols_tracks_multiple_imports() {
    // index.js imports {foo, bar} from ./provider.js
    // provider.js exports foo, bar, baz
    // Only foo and bar should be in provider's usedSymbols
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./provider.js" imports [("foo", "foo", false), ("bar", "bar", false)]
          from "provider.js" provides [("foo", "foo", false), ("bar", "bar", false), ("baz", "baz", false)] {}
      }
    };

    ctx.assert_asset_used_symbols("provider.js", &["foo", "bar"]);
  }

  #[test]
  fn used_symbols_tracks_through_barrel_reexport() {
    // index.js imports {foo} from ./barrel.js
    // barrel.js re-exports {foo} from ./provider.js (weak)
    // provider.js exports foo (strong)
    // Both barrel.js and provider.js should have foo in their usedSymbols
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$barrel$re_export$foo", "foo", false)]
          from "barrel.js" provides [("$barrel$re_export$foo", "foo", true)] {
          dep "./provider.js" imports [("foo", "foo", true)]
            from "provider.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    ctx.assert_asset_used_symbols("provider.js", &["foo"]);
    ctx.assert_asset_used_symbols("barrel.js", &["foo"]);
  }

  #[test]
  fn used_symbols_tracks_renamed_reexport() {
    // index.js imports {renamedFoo} from ./barrel.js
    // barrel.js: export {foo as renamedFoo} from './provider.js'
    // provider.js exports foo (strong)
    // barrel.js should have renamedFoo, provider.js should have foo
    //
    // The dep from barrel→provider has local="$barrel$re_export$renamedFoo"
    // which matches the barrel asset's symbol local, linking the re-export chain.
    // The exported name on the dep is "foo" (the original name from the provider).
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("$barrel$re_export$renamedFoo", "renamedFoo", false)]
          from "barrel.js" provides [("$barrel$re_export$renamedFoo", "renamedFoo", true)] {
          dep "./provider.js" imports [("$barrel$re_export$renamedFoo", "foo", true)]
            from "provider.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    ctx.assert_asset_used_symbols("provider.js", &["foo"]);
    ctx.assert_asset_used_symbols("barrel.js", &["renamedFoo"]);
  }

  #[test]
  fn used_symbols_tracks_star_reexport() {
    // index.js imports {foo} from ./barrel.js
    // barrel.js: export * from './foo.js'; export * from './bar.js'
    // foo.js exports foo
    // bar.js exports bar
    // foo.js should have {foo}, bar.js should have nothing (bar not imported)
    // barrel.js should have {foo} (forwarded through star re-export)
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("foo", "foo", false)]
          from "barrel.js" provides [("*", "*", true)] {
          dep "./foo.js" imports [("*", "*", true)]
            from "foo.js" provides [("foo", "foo", false)] {}
          dep "./bar.js" imports [("*", "*", true)]
            from "bar.js" provides [("bar", "bar", false)] {}
        }
      }
    };

    ctx.assert_asset_used_symbols("foo.js", &["foo"]);
    ctx.assert_asset_used_symbols("bar.js", &[]);
    ctx.assert_asset_used_symbols("barrel.js", &["foo"]);
  }

  #[test]
  fn used_symbols_tracks_namespace_reexport() {
    // index.js imports {ns} from ./barrel.js
    // barrel.js: export * as ns from './provider.js'
    // provider.js exports foo
    // provider.js should have {*} (entire namespace used)
    // barrel.js should have {ns}
    //
    // For namespace re-exports, the dep symbol has exported="*", local="ns"
    // The tuple format is (local, exported, is_weak)
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./barrel.js" imports [("ns", "ns", false)]
          from "barrel.js" provides [("ns", "ns", true)] {
          dep "./provider.js" imports [("ns", "*", false)]
            from "provider.js" provides [("foo", "foo", false)] {}
        }
      }
    };

    ctx.assert_asset_used_symbols("provider.js", &["*"]);
    ctx.assert_asset_used_symbols("barrel.js", &["ns"]);
  }

  #[test]
  fn used_symbols_available_on_finalized_tracker() {
    // Verify that usedSymbols survives finalization and is accessible via the public API
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./provider.js" imports [("foo", "foo", false)]
          from "provider.js" provides [("foo", "foo", false), ("bar", "bar", false)] {}
      }
    };

    let provider_id = ctx.assets.get("provider.js").unwrap().id.clone();
    let index_id = ctx.assets.get("index.js").unwrap().id.clone();

    let finalized = ctx.assert_finalize_ok();

    let provider_used = finalized
      .get_used_symbols_for_asset(&provider_id)
      .expect("provider.js should have used symbols");
    assert_eq!(
      *provider_used,
      HashSet::from(["foo".to_string()]),
      "Only foo should be used from provider.js"
    );

    // Entry asset has no exports, so usedSymbols is empty
    assert_eq!(
      finalized.get_used_symbols_for_asset(&index_id),
      None,
      "index.js should have no used symbols (no exports)"
    );
  }

  #[test]
  fn finalize_excluded_dep_has_empty_used_symbols_up() {
    // index.js imports {f} from lib.js
    // lib.js re-exports f from lib1.js (weak) and b from lib2.js (weak)
    // Nobody imports b → the lib.js → lib2.js dependency should have empty usedSymbolsUp
    // (making it an exclude candidate when lib2.js has side_effects: false)
    let ctx = symbol_tracker_test! {
      entry "index.js" provides [] {
        dep "./lib.js" imports [("$lib$re_export$f", "f", false)]
          from "lib.js" provides [("$lib$re_export$f", "f", true), ("$lib$re_export$b", "b", true)] {
          dep "./lib1.js" imports [("$lib$re_export$f", "f", true)]
            from "lib1.js" provides [("f", "f", false)] {}
          dep "./lib2.js" imports [("$lib$re_export$b", "b", true)]
            from "lib2.js" provides [("b", "b", false)] {}
        }
      }
    };

    // lib1.js should have f in its used symbols, lib2.js should have nothing
    ctx.assert_asset_used_symbols("lib1.js", &["f"]);
    ctx.assert_asset_used_symbols("lib2.js", &[]);

    let dep_lib1 = ctx.deps.get("./lib1.js").unwrap().clone();
    let dep_lib2 = ctx.deps.get("./lib2.js").unwrap().clone();
    let finalized = ctx.assert_finalize_ok();

    // lib1.js dep should have f resolved (it IS used)
    let lib1_used = finalized.get_used_symbols_for_dependency(&dep_lib1.id);
    assert!(
      lib1_used.is_some_and(|u| !u.is_empty()),
      "lib1.js dep should have resolved symbols (f is used)"
    );

    // lib2.js dep should have empty usedSymbolsUp (b is NOT used)
    let lib2_used = finalized.get_used_symbols_for_dependency(&dep_lib2.id);
    assert!(
      lib2_used.is_none() || lib2_used.unwrap().is_empty(),
      "lib2.js dep should have empty usedSymbolsUp (b is not used, exclude candidate)"
    );
  }
}
