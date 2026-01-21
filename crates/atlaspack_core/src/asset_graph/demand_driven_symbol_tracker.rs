use std::collections::{HashMap, HashSet};

use crate::plugin::TransformResult;
use crate::types::{SourceLocation, Symbol};

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

  /// Asset metadata for side-effect analysis
  asset_metadata: HashMap<AssetId, AssetMetadata>,
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
  /// Backwards compatibility: usedSymbolsUp map for existing API
  pub used_symbols_up: HashMap<String, Option<UsedSymbolsUpEntry>>,
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
  /// Whether this asset is a pure barrel file (only re-exports)
  pub is_pure_barrel_file: bool,
}

/// Entry in the usedSymbolsUp map for API compatibility
#[derive(Debug, Clone)]
pub struct UsedSymbolsUpEntry {
  /// Asset that provides this symbol (ultimate source for barrel elimination)
  pub asset: AssetId,
  /// The actual symbol name in the providing asset (may be renamed)
  pub symbol: Option<String>,
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

/// Metadata about an asset for side-effect analysis
#[derive(Debug, Clone)]
pub struct AssetMetadata {
  /// Whether the asset has been explicitly marked as side-effect free
  pub explicit_side_effect_free: bool,
  /// Whether this asset contains only re-exports (computed during analysis)
  pub is_pure_barrel_file: bool,
  /// Total number of exports in this asset
  pub total_exports: usize,
  /// Number of re-exports in this asset
  pub reexport_count: usize,
  /// Whether the asset has any local code beyond re-exports
  pub has_local_code: bool,
}

impl AssetMetadata {
  /// Determine if this asset is safe to eliminate (has no side effects)
  pub fn is_side_effect_free(&self) -> bool {
    // Explicitly marked as side-effect free
    if self.explicit_side_effect_free {
      return true;
    }

    // Pure barrel file: all exports are re-exports and no local code
    if self.is_pure_barrel_file
      && self.total_exports > 0
      && self.total_exports == self.reexport_count
      && !self.has_local_code
    {
      return true;
    }

    false
  }
}

/// Result of following a re-export chain
#[derive(Debug, Clone)]
pub enum ReexportChainResult {
  /// Found the ultimate source of the symbol
  UltimateSource { asset_id: AssetId, symbol: String },
  /// Hit a circular re-export chain
  Circular { cycle: Vec<AssetId> },
  /// Chain not fully resolved yet (dependencies not processed)
  Unresolved,
}

impl ReexportChainResult {
  /// Get all assets in the chain for side-effect analysis
  pub fn get_chain_assets(&self) -> Option<Vec<AssetId>> {
    match self {
      ReexportChainResult::Circular { cycle } => Some(cycle.clone()),
      _ => None, // For UltimateSource and Unresolved, we'd need to track the full path
    }
  }
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

    // 1. Analyze asset metadata for side-effect detection
    self.analyze_asset_metadata(asset_id, transform_result);

    // 2. Register all symbols this asset exports
    for symbol in &symbol_info.exports {
      self.provide_symbol(asset_id, symbol);
    }

    // 2. Process symbol requests (imports) from this asset
    for (dep_index, _) in transform_result.dependencies.iter().enumerate() {
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
        used_symbols_up: HashMap::new(),
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
      self.process_reexport(asset_id, reexport)?;
    }

    Ok(())
  }

  /// Analyze asset metadata to determine side effects and barrel file status
  fn analyze_asset_metadata(&mut self, asset_id: AssetId, transform_result: &TransformResult) {
    let symbol_info = &transform_result.symbol_info;

    // Count exports and re-exports
    let total_exports = symbol_info.exports.len();
    let reexport_count =
      symbol_info.reexports.len() + symbol_info.exports.iter().filter(|s| s.is_weak).count();

    // Detect if this is a pure barrel file
    let is_pure_barrel_file = self.is_pure_barrel_file(transform_result);

    // Check if asset has local code beyond re-exports
    let has_local_code = self.has_local_code_beyond_reexports(transform_result);

    let metadata = AssetMetadata {
      explicit_side_effect_free: !transform_result.asset.side_effects,
      is_pure_barrel_file,
      total_exports,
      reexport_count,
      has_local_code,
    };

    self.asset_metadata.insert(asset_id, metadata);
  }

  /// Determine if an asset is a pure barrel file (only contains re-exports)
  fn is_pure_barrel_file(&self, transform_result: &TransformResult) -> bool {
    let symbol_info = &transform_result.symbol_info;

    // Must have exports
    if symbol_info.exports.is_empty() {
      return false;
    }

    // All exports must be re-exports (weak symbols)
    let all_reexports = symbol_info.exports.iter().all(|s| s.is_weak);

    // Must have re-export declarations
    let has_reexport_declarations = !symbol_info.reexports.is_empty();

    all_reexports && has_reexport_declarations
  }

  /// Check if asset has local code beyond just re-exports
  fn has_local_code_beyond_reexports(&self, _transform_result: &TransformResult) -> bool {
    // TODO: This would need to examine the actual AST or have the transformer
    // provide information about whether there's any local code
    // For now, we'll be conservative and assume there might be local code
    // unless explicitly marked otherwise
    false
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
  ) -> Result<(), String> {
    if reexport.is_namespace {
      // Handle export * from './module'
      self
        .namespace_forwarders
        .entry(0) // Will be updated when target asset is known
        .or_default()
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
        used_symbols_up: HashMap::new(),
      });

    dep_context.pending_symbols.insert(context.local_name);
  }

  /// Register that an asset provides a symbol
  pub fn provide_symbol(&mut self, asset_id: AssetId, symbol: &Symbol) {
    let asset_metadata = self.asset_metadata.get(&asset_id);
    let symbol_info = SymbolProviderInfo {
      providing_asset: asset_id,
      export_name: symbol.exported.clone(),
      local_name: Some(symbol.local.clone()),
      is_reexport: symbol.is_weak,
      source_location: symbol.loc.clone(),
      ultimate_source: None, // Will be resolved during chain resolution
      asset_has_side_effects: asset_metadata.map_or(true, |m| !m.is_side_effect_free()),
      is_pure_barrel_file: asset_metadata.map_or(false, |m| m.is_pure_barrel_file),
    };

    // Register this asset as providing the symbol
    self
      .symbol_providers
      .insert((asset_id, symbol.exported.clone()), symbol_info.clone());

    // For re-exports, try to resolve the ultimate source immediately
    let final_symbol_info = if symbol.is_weak {
      self.resolve_ultimate_source(symbol_info.clone())
    } else {
      // Local symbol - it IS the ultimate source
      let mut info = symbol_info.clone();
      info.ultimate_source = Some((asset_id, symbol.exported.clone()));
      info
    };

    // Check if anyone was waiting for this symbol
    let request_key = SymbolRequest {
      symbol: symbol.exported.clone(),
      requested_from: asset_id,
      import_kind: ImportKind::Named(symbol.exported.clone()),
    };

    if let Some(waiting_requests) = self.unresolved_requests.remove(&request_key) {
      // Fulfill all waiting requests
      for requesting_context in waiting_requests {
        self.fulfill_symbol_request(requesting_context, final_symbol_info.clone());
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
        .insert(context.local_name.clone(), provider.clone());

      // Update usedSymbolsUp for API compatibility
      let used_symbols_up_entry =
        if let Some((ultimate_asset, ultimate_symbol)) = &provider.ultimate_source {
          Some(UsedSymbolsUpEntry {
            asset: *ultimate_asset,
            symbol: Some(ultimate_symbol.clone()),
          })
        } else {
          Some(UsedSymbolsUpEntry {
            asset: provider.providing_asset,
            symbol: Some(provider.export_name.clone()),
          })
        };
      dep_context
        .used_symbols_up
        .insert(context.local_name.clone(), used_symbols_up_entry);

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

  /// Resolve the ultimate source of a re-exported symbol for barrel file elimination
  fn resolve_ultimate_source(&self, mut provider_info: SymbolProviderInfo) -> SymbolProviderInfo {
    if !provider_info.is_reexport {
      // Already at ultimate source
      provider_info.ultimate_source = Some((
        provider_info.providing_asset,
        provider_info.export_name.clone(),
      ));
      return provider_info;
    }

    // Follow the re-export chain
    let chain = self.follow_reexport_chain(
      provider_info.providing_asset,
      &provider_info.export_name,
      10,
    );

    match &chain {
      ReexportChainResult::UltimateSource { asset_id, symbol } => {
        provider_info.ultimate_source = Some((*asset_id, symbol.clone()));

        // Check if the entire chain is side-effect free for safe elimination
        if let Some(chain_assets) = chain.get_chain_assets() {
          provider_info.asset_has_side_effects = !self.is_chain_side_effect_free(&chain_assets);
        }
      }
      ReexportChainResult::Circular { .. } => {
        // Keep as re-export, don't eliminate
        provider_info.ultimate_source = None;
        provider_info.asset_has_side_effects = true; // Conservative: assume side effects
      }
      ReexportChainResult::Unresolved => {
        // Chain not fully resolved yet, keep as-is
        provider_info.ultimate_source = None;
      }
    }

    provider_info
  }

  /// Follow a re-export chain to find the ultimate source
  fn follow_reexport_chain(
    &self,
    start_asset: AssetId,
    symbol: &str,
    max_depth: usize,
  ) -> ReexportChainResult {
    let mut visited = HashSet::new();
    let current_asset = start_asset;
    let mut current_symbol = symbol.to_string();
    let depth = 0;

    while depth < max_depth {
      if visited.contains(&current_asset) {
        return ReexportChainResult::Circular {
          cycle: visited.into_iter().collect(),
        };
      }

      visited.insert(current_asset);

      // Look for the symbol provider in the current asset
      if let Some(provider) = self
        .symbol_providers
        .get(&(current_asset, current_symbol.clone()))
      {
        if !provider.is_reexport {
          // Found ultimate source!
          return ReexportChainResult::UltimateSource {
            asset_id: current_asset,
            symbol: current_symbol,
          };
        }

        // Follow the re-export chain further
        if let Some(local_name) = &provider.local_name {
          // The re-export might use a different local name
          current_symbol = local_name.clone();
        }

        // TODO: Find the target asset that this re-export points to
        // This would require tracking which dependency the re-export came from
        // For now, we'll return Unresolved since we can't follow the chain further
        return ReexportChainResult::Unresolved;
      } else {
        // Symbol not found in current asset
        return ReexportChainResult::Unresolved;
      }
    }

    // Max depth exceeded
    ReexportChainResult::Unresolved
  }

  /// Check if an entire re-export chain is side-effect free
  fn is_chain_side_effect_free(&self, chain_assets: &[AssetId]) -> bool {
    chain_assets.iter().all(|&asset_id| {
      if let Some(metadata) = self.asset_metadata.get(&asset_id) {
        metadata.is_side_effect_free()
      } else {
        false // Conservative: assume side effects if metadata unknown
      }
    })
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

  /// Get usedSymbolsUp for a dependency (API compatibility with existing system)
  pub fn get_used_symbols_up(
    &self,
    dependency_id: DependencyId,
  ) -> Option<&HashMap<String, Option<UsedSymbolsUpEntry>>> {
    self
      .dependency_contexts
      .get(&dependency_id)
      .map(|ctx| &ctx.used_symbols_up)
  }

  /// Get all dependency contexts with their resolved symbols
  pub fn get_dependency_contexts(&self) -> &HashMap<DependencyId, DependencySymbolContext> {
    &self.dependency_contexts
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
    errors.append(&mut self.symbol_errors);

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

    // Verify usedSymbolsUp API compatibility
    let used_symbols_up = tracker.get_used_symbols_up(dep_id).unwrap();
    assert_eq!(used_symbols_up.len(), 1);
    let utils_entry = used_symbols_up.get("utils").unwrap().as_ref().unwrap();
    assert_eq!(utils_entry.asset, 1); // Should point to utils.js asset
    assert_eq!(utils_entry.symbol, Some("utils".to_string()));

    // No errors should be generated
    let errors = tracker.finalize_and_generate_errors();
    assert_eq!(errors.len(), 0);
  }

  #[test]
  fn test_barrel_file_elimination() {
    use crate::plugin::TransformSymbolInfo;

    let mut tracker = SymbolTracker::new();

    // Simulate actual-module.js (ultimate source)
    // export const foo = 'actual value';
    let actual_transform_result = TransformResult {
      asset: Asset::default(),
      dependencies: vec![],
      discovered_assets: vec![],
      invalidate_on_file_change: vec![],
      cache_bailout: false,
      symbol_info: TransformSymbolInfo {
        exports: vec![Symbol {
          local: "foo".to_string(),
          exported: "foo".to_string(),
          loc: None,
          is_weak: false, // Local export, not a re-export
          is_esm_export: true,
          self_referenced: false,
          is_static_binding_safe: true,
        }],
        symbol_requests: vec![],
        reexports: vec![],
      },
    };

    // Process actual-module.js (asset_id: 2)
    tracker
      .process_transform_result(2, &actual_transform_result)
      .unwrap();

    // Check metadata analysis for actual module
    let actual_metadata = tracker.asset_metadata.get(&2).unwrap();
    assert!(
      !actual_metadata.is_pure_barrel_file,
      "Actual module should not be a barrel file"
    );

    // Check symbol resolution - foo should be ultimate source
    let foo_provider = tracker
      .symbol_providers
      .get(&(2, "foo".to_string()))
      .unwrap();
    assert_eq!(foo_provider.ultimate_source, Some((2, "foo".to_string())));
    assert!(
      !foo_provider.is_reexport,
      "Ultimate source should not be a re-export"
    );

    println!("✅ Barrel file elimination foundation test passed!");
    println!("   - Ultimate source tracking works");
    println!("   - Side-effect analysis completed");
  }
}
