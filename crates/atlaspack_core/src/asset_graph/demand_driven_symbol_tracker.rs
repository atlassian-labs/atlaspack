use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::plugin::{TransformResult, TransformSymbolInfo};
use crate::types::{Asset, Dependency, SourceLocation, Symbol};

pub type AssetId = usize;
pub type DependencyId = usize;

/// Central tracker for demand-driven symbol resolution during asset graph construction
#[derive(Debug, Default)]
pub struct SymbolTracker {
  /// Global registry of unresolved symbol requests
  unresolved_requests: HashMap<SymbolRequest, Vec<RequestingContext>>,

  /// Reverse index: when we find a symbol, who was waiting for it?
  symbol_providers: HashMap<(AssetId, String), SymbolProviderInfo>,

  /// Dependency-scoped tracking
  dependency_contexts: HashMap<DependencyId, DependencySymbolContext>,

  /// Handle circular dependencies separately
  circular_requests: Vec<SymbolRequest>,

  /// Track namespace re-exports (export *)
  namespace_forwarders: HashMap<AssetId, Vec<AssetId>>,

  /// Errors encountered during symbol resolution
  symbol_errors: Vec<SymbolError>,
}

/// A request for a specific symbol from a specific asset
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolRequest {
  /// The symbol name being requested
  pub symbol: String,
  /// The asset that should provide this symbol
  pub requested_from: AssetId,
  /// How the symbol was imported (for error messages and resolution logic)
  pub import_kind: ImportKind,
}

/// Information about who is requesting a symbol
#[derive(Debug, Clone)]
pub struct RequestingContext {
  /// The asset that is importing the symbol
  pub requesting_asset: AssetId,
  /// The dependency that connects requesting asset to target
  pub dependency_id: DependencyId,
  /// The local name this symbol will have in the requesting asset
  pub local_name: String,
  /// Whether this is a namespace import (import * as foo)
  pub is_namespace: bool,
  /// Whether this is a type-only import
  pub is_type_only: bool,
  /// Source location of the import for error reporting
  pub source_location: Option<SourceLocation>,
}

/// Symbol resolution state for a specific dependency
#[derive(Debug, Clone)]
pub struct DependencySymbolContext {
  pub dependency_id: DependencyId,
  /// The target asset (None until dependency is resolved)
  pub target_asset: Option<AssetId>,
  /// Symbols we're still waiting to resolve
  pub pending_symbols: HashSet<String>,
  /// Symbols we have resolved
  pub resolved_symbols: HashMap<String, SymbolProviderInfo>,
}

/// Information about an asset that provides a symbol
#[derive(Debug, Clone)]
pub struct SymbolProviderInfo {
  /// The asset that provides this symbol
  pub providing_asset: AssetId,
  /// The exported name of the symbol
  pub export_name: String,
  /// The local name within the providing asset (for re-exports)
  pub local_name: Option<String>,
  /// Whether this is a re-export (weak symbol)
  pub is_reexport: bool,
  /// Source location of the export
  pub source_location: Option<SourceLocation>,
  /// For barrel file elimination: the ultimate source of this symbol
  pub ultimate_source: Option<(AssetId, String)>,
  /// Whether the providing asset has side effects
  pub asset_has_side_effects: bool,
}

/// Different ways a symbol can be imported
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportKind {
  /// import { foo } from './bar'
  Named(String),
  /// import * as foo from './bar'  
  Namespace,
  /// import foo from './bar'
  Default,
}

/// Different types of symbol resolution errors
#[derive(Debug, Clone)]
pub enum SymbolError {
  /// Symbol not found in target asset
  NotFound {
    symbol: String,
    requesting_asset: AssetId,
    target_asset: AssetId,
    source_location: Option<SourceLocation>,
    suggestion: Option<String>,
  },
  /// Multiple assets provide the same symbol (export * ambiguity)
  Ambiguous {
    symbol: String,
    requesting_asset: AssetId,
    providers: Vec<AssetId>,
    source_location: Option<SourceLocation>,
  },
  /// Circular dependency detected
  Circular { symbol: String, cycle: Vec<AssetId> },
}

impl SymbolTracker {
  pub fn new() -> Self {
    Self::default()
  }

  /// Process a TransformResult and update symbol tracking
  /// This is the main integration point called after asset transformation
  pub fn process_transform_result(
    &mut self,
    asset_id: AssetId,
    transform_result: &TransformResult,
  ) -> Result<(), String> {
    let symbol_info = &transform_result.symbol_info;

    // 1. Register all symbols this asset exports
    for symbol in &symbol_info.exports {
      self.provide_symbol(asset_id, symbol);
    }

    // 2. Process symbol requests (imports) from this asset
    for (dep_index, dependency) in transform_result.dependencies.iter().enumerate() {
      // Find symbol requests for this dependency
      let requests_for_this_dep: Vec<_> = symbol_info
        .symbol_requests
        .iter()
        .filter(|req| req.dependency_index == dep_index)
        .collect();

      if requests_for_this_dep.is_empty() {
        continue;
      }

      // Create dependency context
      let dep_id = self.create_dependency_id(asset_id, dep_index);
      let mut dep_context = DependencySymbolContext {
        dependency_id: dep_id,
        target_asset: None, // Will be set when dependency is resolved
        pending_symbols: HashSet::new(),
        resolved_symbols: HashMap::new(),
      };

      // Add each symbol request
      for request in requests_for_this_dep {
        let symbol_request = crate::asset_graph::SymbolRequest {
          symbol: request.symbol.clone(),
          requested_from: 0, // Will be updated when target asset is known
          import_kind: self.convert_import_kind(&request.import_kind),
        };

        let context = RequestingContext {
          requesting_asset: asset_id,
          dependency_id: dep_id,
          local_name: request.local_name.clone(),
          is_namespace: matches!(request.import_kind, crate::plugin::ImportKind::Namespace),
          is_type_only: false, // TODO: Add type-only support
          source_location: request.source_location.clone(),
        };

        dep_context
          .pending_symbols
          .insert(request.local_name.clone());
        self.add_request(symbol_request, context);
      }

      self.dependency_contexts.insert(dep_id, dep_context);
    }

    // 3. Process re-exports for barrel file handling
    for reexport in &symbol_info.reexports {
      self.process_reexport(asset_id, reexport, &transform_result.dependencies)?;
    }

    Ok(())
  }

  /// Create a unique dependency ID from asset ID and dependency index
  fn create_dependency_id(&self, asset_id: AssetId, dep_index: usize) -> DependencyId {
    // Simple hash combination - in real implementation might want something more sophisticated
    asset_id * 10000 + dep_index
  }

  /// Convert plugin ImportKind to our internal ImportKind
  fn convert_import_kind(&self, kind: &crate::plugin::ImportKind) -> ImportKind {
    match kind {
      crate::plugin::ImportKind::Named(name) => ImportKind::Named(name.clone()),
      crate::plugin::ImportKind::Namespace => ImportKind::Namespace,
      crate::plugin::ImportKind::Default => ImportKind::Default,
    }
  }

  /// Process re-export information
  fn process_reexport(
    &mut self,
    asset_id: AssetId,
    reexport: &crate::plugin::ReexportInfo,
    dependencies: &[Dependency],
  ) -> Result<(), String> {
    let dep_id = self.create_dependency_id(asset_id, reexport.dependency_index);

    if reexport.is_namespace {
      // Handle export * from './module'
      self
        .namespace_forwarders
        .entry(0) // Will be updated when target asset is known
        .or_insert_with(Vec::new)
        .push(asset_id);
    } else if let Some(symbols) = &reexport.symbols {
      // Handle export { foo, bar } from './module'
      for symbol in symbols {
        // Create a symbol that re-exports from the dependency
        let reexported_symbol = Symbol {
          local: format!("{}#{}", asset_id, symbol), // Unique local name
          exported: symbol.clone(),
          loc: None,     // TODO: Get from reexport info
          is_weak: true, // Re-exports are weak
          is_esm_export: true,
          self_referenced: false,
          is_static_binding_safe: true,
        };

        self.provide_symbol(asset_id, &reexported_symbol);
      }
    }

    Ok(())
  }

  /// Update symbol tracker when a dependency is resolved to a target asset
  pub fn resolve_dependency(&mut self, dependency_id: DependencyId, target_asset_id: AssetId) {
    // Update dependency context
    if let Some(dep_context) = self.dependency_contexts.get_mut(&dependency_id) {
      dep_context.target_asset = Some(target_asset_id);
    }

    // Update any unresolved requests to point to the correct target asset
    let mut requests_to_update = Vec::new();
    for (request, contexts) in &self.unresolved_requests {
      for context in contexts {
        if context.dependency_id == dependency_id {
          let mut updated_request = request.clone();
          updated_request.requested_from = target_asset_id;
          requests_to_update.push((request.clone(), updated_request, context.clone()));
        }
      }
    }

    // Apply updates
    for (old_request, new_request, context) in requests_to_update {
      if let Some(contexts) = self.unresolved_requests.get_mut(&old_request) {
        contexts.retain(|c| c.dependency_id != dependency_id);
        if contexts.is_empty() {
          self.unresolved_requests.remove(&old_request);
        }
      }

      self
        .unresolved_requests
        .entry(new_request)
        .or_insert_with(Vec::new)
        .push(context);
    }
  }

  /// Record a symbol request during asset transformation
  pub fn add_request(&mut self, request: SymbolRequest, context: RequestingContext) {
    // Check if this might be a circular dependency
    if self.detect_circular_dependency(&request) {
      self.circular_requests.push(request);
      return;
    }

    // Add to unresolved requests
    self
      .unresolved_requests
      .entry(request)
      .or_insert_with(Vec::new)
      .push(context.clone());

    // Update dependency context
    let dep_context = self
      .dependency_contexts
      .entry(context.dependency_id)
      .or_insert_with(|| DependencySymbolContext {
        dependency_id: context.dependency_id,
        target_asset: None,
        pending_symbols: HashSet::new(),
        resolved_symbols: HashMap::new(),
      });

    dep_context.pending_symbols.insert(context.local_name);
  }

  /// Register that an asset provides a symbol
  pub fn provide_symbol(&mut self, asset_id: AssetId, symbol: &Symbol) {
    let symbol_info = SymbolProviderInfo {
      providing_asset: asset_id,
      export_name: symbol.exported.clone(),
      local_name: Some(symbol.local.clone()),
      is_reexport: symbol.is_weak,
      source_location: symbol.loc.clone(),
      ultimate_source: None,         // Will be resolved later for re-exports
      asset_has_side_effects: false, // TODO: Get from asset metadata
    };

    // Register this asset as providing the symbol
    self
      .symbol_providers
      .insert((asset_id, symbol.exported.clone()), symbol_info.clone());

    // Check if anyone was waiting for this symbol
    let request_key = SymbolRequest {
      symbol: symbol.exported.clone(),
      requested_from: asset_id,
      import_kind: ImportKind::Named(symbol.exported.clone()),
    };

    if let Some(waiting_requests) = self.unresolved_requests.remove(&request_key) {
      // Fulfill all waiting requests
      for requesting_context in waiting_requests {
        self.fulfill_symbol_request(requesting_context, symbol_info.clone());
      }
    }
  }

  /// Fulfill a symbol request with a provider
  fn fulfill_symbol_request(&mut self, context: RequestingContext, provider: SymbolProviderInfo) {
    // Update the dependency context
    if let Some(dep_context) = self.dependency_contexts.get_mut(&context.dependency_id) {
      dep_context.pending_symbols.remove(&context.local_name);
      dep_context
        .resolved_symbols
        .insert(context.local_name, provider.clone());

      // Mark dependency as resolved if all symbols are fulfilled
      if dep_context.pending_symbols.is_empty() {
        self.finalize_dependency_symbols(context.dependency_id);
      }
    }
  }

  /// Finalize symbol resolution for a dependency
  fn finalize_dependency_symbols(&mut self, _dependency_id: DependencyId) {
    // TODO: This is where we would update the asset graph with resolved symbols
    // and potentially trigger barrel file elimination logic
  }

  /// Detect if a symbol request would create a circular dependency
  fn detect_circular_dependency(&self, _request: &SymbolRequest) -> bool {
    // Simple cycle detection: check if the target asset also requests symbols from the requesting asset
    // TODO: Implement more sophisticated cycle detection
    false
  }

  /// Get all unresolved symbol requests (for error reporting)
  pub fn get_unresolved_requests(&self) -> &HashMap<SymbolRequest, Vec<RequestingContext>> {
    &self.unresolved_requests
  }

  /// Get all symbol resolution errors
  pub fn get_symbol_errors(&self) -> &[SymbolError] {
    &self.symbol_errors
  }

  /// Generate final symbol errors for any unresolved requests
  pub fn finalize_and_generate_errors(&mut self) -> Vec<SymbolError> {
    let mut errors = Vec::new();

    // Convert unresolved requests to errors
    for (request, contexts) in &self.unresolved_requests {
      for context in contexts {
        errors.push(SymbolError::NotFound {
          symbol: request.symbol.clone(),
          requesting_asset: context.requesting_asset,
          target_asset: request.requested_from,
          source_location: context.source_location.clone(),
          suggestion: None, // TODO: Implement symbol suggestions
        });
      }
    }

    // Add any accumulated errors
    errors.extend(self.symbol_errors.drain(..));

    errors
  }
}

impl SymbolRequest {
  /// Create a symbol request for a specific symbol from a specific asset
  pub fn for_symbol(asset_id: AssetId, symbol: String) -> Self {
    Self {
      symbol: symbol.clone(),
      requested_from: asset_id,
      import_kind: ImportKind::Named(symbol),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_symbol_request_fulfillment() {
    let mut tracker = SymbolTracker::new();

    // Create a symbol request
    let request = SymbolRequest {
      symbol: "foo".to_string(),
      requested_from: 1, // target asset id
      import_kind: ImportKind::Named("foo".to_string()),
    };

    let context = RequestingContext {
      requesting_asset: 0, // requesting asset id
      dependency_id: 0,
      local_name: "foo".to_string(),
      is_namespace: false,
      is_type_only: false,
      source_location: None,
    };

    // Add the request
    tracker.add_request(request, context);

    // Should have one unresolved request
    assert_eq!(tracker.unresolved_requests.len(), 1);

    // Now provide the symbol
    let symbol = Symbol {
      local: "foo".to_string(),
      exported: "foo".to_string(),
      loc: None,
      is_weak: false,
      is_esm_export: true,
      self_referenced: false,
      is_static_binding_safe: true,
    };

    tracker.provide_symbol(1, &symbol);

    // Request should be resolved
    assert_eq!(tracker.unresolved_requests.len(), 0);
    assert_eq!(tracker.symbol_providers.len(), 1);
  }

  #[test]
  fn test_unresolved_symbol_error_generation() {
    let mut tracker = SymbolTracker::new();

    // Add an unresolved request
    let request = SymbolRequest {
      symbol: "missing".to_string(),
      requested_from: 1,
      import_kind: ImportKind::Named("missing".to_string()),
    };

    let context = RequestingContext {
      requesting_asset: 0,
      dependency_id: 0,
      local_name: "missing".to_string(),
      is_namespace: false,
      is_type_only: false,
      source_location: None,
    };

    tracker.add_request(request, context);

    // Generate errors
    let errors = tracker.finalize_and_generate_errors();

    assert_eq!(errors.len(), 1);
    match &errors[0] {
      SymbolError::NotFound {
        symbol,
        requesting_asset,
        target_asset,
        ..
      } => {
        assert_eq!(symbol, "missing");
        assert_eq!(*requesting_asset, 0);
        assert_eq!(*target_asset, 1);
      }
      _ => panic!("Expected NotFound error"),
    }
  }

  #[test]
  fn test_demand_driven_symbol_resolution_integration() {
    use crate::plugin::{
      ImportKind as PluginImportKind, SymbolRequest as PluginSymbolRequest, TransformSymbolInfo,
    };
    use crate::types::Dependency;

    let mut tracker = SymbolTracker::new();

    // Simulate app.js transformation result
    // import { utils } from './utils.js';
    // export const app = 'main';
    let app_transform_result = TransformResult {
      asset: Asset::default(), // Would be populated in real scenario
      dependencies: vec![
        Dependency::default(), // utils.js dependency
      ],
      discovered_assets: vec![],
      invalidate_on_file_change: vec![],
      cache_bailout: false,
      symbol_info: TransformSymbolInfo {
        exports: vec![Symbol {
          local: "app".to_string(),
          exported: "app".to_string(),
          loc: None,
          is_weak: false,
          is_esm_export: true,
          self_referenced: false,
          is_static_binding_safe: true,
        }],
        symbol_requests: vec![PluginSymbolRequest {
          symbol: "utils".to_string(),
          dependency_index: 0, // First dependency (utils.js)
          local_name: "utils".to_string(),
          import_kind: PluginImportKind::Named("utils".to_string()),
          source_location: None,
        }],
        reexports: vec![],
      },
    };

    // Process app.js
    tracker
      .process_transform_result(0, &app_transform_result)
      .unwrap();

    // Should have one unresolved request for 'utils'
    assert_eq!(tracker.unresolved_requests.len(), 1);
    assert_eq!(tracker.symbol_providers.len(), 1); // app export

    // Simulate dependency resolution: dependency 0 points to asset 1 (utils.js)
    let dep_id = tracker.create_dependency_id(0, 0);
    tracker.resolve_dependency(dep_id, 1);

    // Now simulate utils.js transformation result
    // export const utils = 'helper';
    let utils_transform_result = TransformResult {
      asset: Asset::default(),
      dependencies: vec![],
      discovered_assets: vec![],
      invalidate_on_file_change: vec![],
      cache_bailout: false,
      symbol_info: TransformSymbolInfo {
        exports: vec![Symbol {
          local: "utils".to_string(),
          exported: "utils".to_string(),
          loc: None,
          is_weak: false,
          is_esm_export: true,
          self_referenced: false,
          is_static_binding_safe: true,
        }],
        symbol_requests: vec![],
        reexports: vec![],
      },
    };

    // Process utils.js
    tracker
      .process_transform_result(1, &utils_transform_result)
      .unwrap();

    // The 'utils' symbol request should now be resolved
    assert_eq!(tracker.unresolved_requests.len(), 0);
    assert_eq!(tracker.symbol_providers.len(), 2); // app + utils exports

    // Verify the dependency context was updated
    let dep_context = tracker.dependency_contexts.get(&dep_id).unwrap();
    assert_eq!(dep_context.target_asset, Some(1));
    assert!(dep_context.pending_symbols.is_empty()); // Should be resolved
    assert_eq!(dep_context.resolved_symbols.len(), 1);

    // No errors should be generated
    let errors = tracker.finalize_and_generate_errors();
    assert_eq!(errors.len(), 0);
  }
}
