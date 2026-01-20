# Demand-Driven Symbol Resolution Design

## Background

Currently, Atlaspack uses a two-pass Symbol Propagation system that runs **after** the asset graph is built:

1. **Down Pass**: Forward symbol requests from importers to dependencies using BFS
2. **Up Pass**: Resolve symbols from exports back to importers using DFS

**Problem**: This requires a separate post-processing step and full graph traversal.

**Goal**: Move to an incremental, demand-driven approach that resolves symbols **during** asset transformation, eliminating the need for post-processing.

## Current Symbol Propagation Architecture

### Key Files

- `packages/core/core/src/SymbolPropagation.ts` - Main JS implementation
- `crates/atlaspack_core/src/asset_graph/propagate_requested_symbols.rs` - Rust implementation
- `packages/core/core/src/requests/AssetGraphRequest.ts` - Integration point

### Current Process

1. Build complete asset graph with basic dependency information
2. Run `propagateSymbolsDown()` - BFS to forward symbol requests
3. Run `propagateSymbolsUp()` - DFS to resolve symbols and build `usedSymbolsUp` map
4. Use `usedSymbolsUp` for barrel file elimination in `BundleGraph.fromAssetGraph()`

### Key Insights from Current System

- **Barrel File Elimination**: Lines 279-462 in `BundleGraph.ts` use symbol resolution data to retarget dependencies directly to ultimate symbol sources
- **Weak Symbols**: `isWeak: true` flag indicates re-exports (barrel files) vs. local definitions
- **Symbol Resolution Tracking**: `usedSymbolsUp` maps symbols to their ultimate providing assets

## Proposed Demand-Driven Approach

### Core Concept

Instead of post-processing, track **symbol requests** during transformation and **fulfill** them as symbols are discovered in later assets.

### Key Data Structures

```rust
struct SymbolTracker {
    // Global registry of unresolved symbol requests
    unresolved_requests: HashMap<SymbolRequest, Vec<RequestingContext>>,

    // Reverse index: when we find a symbol, who was waiting for it?
    symbol_providers: HashMap<(AssetId, Symbol), SymbolProviderInfo>,

    // Dependency-scoped tracking
    dependency_contexts: HashMap<DependencyId, DependencySymbolContext>,

    // Handle circular dependencies separately
    circular_requests: Vec<SymbolRequest>,

    // Track namespace re-exports (export *)
    namespace_forwarders: HashMap<AssetId, Vec<AssetId>>,
}

struct SymbolRequest {
    symbol: Symbol,
    requested_from: AssetId,  // The dependency target
    import_specifier: ImportSpecifier, // How it was imported
}

struct RequestingContext {
    requesting_asset: AssetId,
    dependency_id: DependencyId,
    local_name: Symbol,
    is_namespace: bool,
    is_type_only: bool,
    source_location: SourceLocation,
}

struct DependencySymbolContext {
    dependency_id: DependencyId,
    target_asset: Option<AssetId>,  // None until dependency resolved
    pending_symbols: HashSet<Symbol>,
    resolved_symbols: HashMap<Symbol, SymbolProviderInfo>,
}

struct SymbolProviderInfo {
    providing_asset: AssetId,
    export_name: Symbol,
    local_name: Option<Symbol>,
    is_reexport: bool,
    is_weak: bool, // true for re-exports, false for local definitions
    source_location: SourceLocation,
    ultimate_source: Option<(AssetId, Symbol)>, // For barrel file elimination
}
```

### Workflow

#### 1. During Asset Transformation - Record Requests

```rust
impl AssetTransformer {
    fn transform_asset(&mut self, asset: Asset) -> TransformResult {
        // Normal transformation...

        // For each import, record symbol requests
        for import in &ast.imports {
            let dep_id = self.create_dependency(&import.source);

            for specifier in &import.specifiers {
                let request = SymbolRequest {
                    symbol: specifier.imported.clone(),
                    requested_from: dep_id.target_asset(),
                    import_specifier: specifier.clone(),
                };

                let context = RequestingContext {
                    requesting_asset: asset.id,
                    dependency_id: dep_id,
                    local_name: specifier.local.clone(),
                    is_namespace: matches!(specifier, ImportSpecifier::Namespace(_)),
                    is_type_only: specifier.is_type_only,
                    source_location: specifier.span,
                };

                // Record the request
                self.symbol_tracker.add_request(request, context);
            }
        }

        // For each export, check if anyone was waiting for it
        for export in &ast.exports {
            self.symbol_tracker.provide_symbol(asset.id, export);
        }

        TransformResult { /* ... */ }
    }
}
```

#### 2. Symbol Provider Registration

```rust
impl SymbolTracker {
    fn provide_symbol(&mut self, asset_id: AssetId, export: &ExportDecl) {
        let symbol_info = SymbolProviderInfo {
            providing_asset: asset_id,
            export_name: export.exported_name(),
            local_name: export.local_name(),
            is_reexport: export.is_reexport(),
            is_weak: export.is_reexport(), // Weak = re-export
            source_location: export.span(),
            ultimate_source: None, // Will be resolved for re-exports
        };

        // Register this asset as providing the symbol
        self.symbol_providers.insert(
            (asset_id, export.exported_name()),
            symbol_info.clone()
        );

        // Check if anyone was waiting for this symbol
        let request_key = SymbolRequest::for_symbol(asset_id, export.exported_name());
        if let Some(waiting_requests) = self.unresolved_requests.remove(&request_key) {
            // Fulfill all waiting requests
            for requesting_context in waiting_requests {
                self.fulfill_symbol_request(requesting_context, symbol_info.clone());
            }
        }
    }

    fn fulfill_symbol_request(&mut self, context: RequestingContext, provider: SymbolProviderInfo) {
        // Update the dependency context
        if let Some(dep_context) = self.dependency_contexts.get_mut(&context.dependency_id) {
            dep_context.pending_symbols.remove(&context.local_name);
            dep_context.resolved_symbols.insert(context.local_name, provider.clone());

            // If this dependency is fully resolved, finalize it
            if dep_context.pending_symbols.is_empty() {
                self.finalize_dependency_symbols(context.dependency_id, dep_context);
            }
        }

        // Update the requesting asset's symbol table
        self.update_asset_symbol_resolution(context.requesting_asset, context.local_name, provider);
    }
}
```

#### 3. Re-export Handling

```rust
impl SymbolTracker {
    fn provide_reexport(&mut self, asset_id: AssetId, reexport: &ReexportDecl) {
        match reexport {
            ReexportDecl::Named { source, specifiers } => {
                // For each re-exported symbol, forward the request
                for spec in specifiers {
                    let forward_request = SymbolRequest {
                        symbol: spec.imported.clone(),
                        requested_from: source.target_asset(),
                        import_specifier: spec.clone(),
                    };

                    // When the source provides this symbol, we'll provide it too
                    self.add_reexport_forwarding(forward_request, asset_id, spec.exported.clone());
                }
            },
            ReexportDecl::Namespace { source } => {
                // For export *, track this forwarding relationship
                self.namespace_forwarders
                    .entry(source.target_asset())
                    .or_insert_with(Vec::new)
                    .push(asset_id);

                // When source asset provides ANY symbol, forward it through this asset
                self.add_namespace_forwarding(asset_id, source.target_asset());
            }
        }
    }

    fn resolve_reexport_chain(&mut self, symbol: &Symbol, chain: &[AssetId]) -> Option<(AssetId, Symbol)> {
        // Follow re-export chain to ultimate source for barrel file elimination
        let mut current_asset = chain[0];
        let mut current_symbol = symbol.clone();

        for &next_asset in &chain[1..] {
            if let Some(provider) = self.symbol_providers.get(&(current_asset, current_symbol.clone())) {
                if provider.is_reexport {
                    current_asset = next_asset;
                    current_symbol = provider.local_name.clone().unwrap_or(current_symbol);
                } else {
                    // Found ultimate source
                    return Some((current_asset, current_symbol));
                }
            } else {
                return None; // Chain broken
            }
        }

        Some((current_asset, current_symbol))
    }
}
```

## Edge Cases and Challenges

### 1. Export \* Ambiguity

**Problem**: Multiple assets provide the same symbol via `export *`

**Solution**:

```rust
fn handle_ambiguous_symbol(&mut self, symbol: Symbol, providers: Vec<AssetId>) {
    match providers.len() {
        0 => { /* Will remain unresolved - error at end */ },
        1 => self.provide_symbol(providers[0], symbol),
        _ => {
            // Check if they're all re-exports of the same ultimate source
            let ultimate_sources: HashSet<_> = providers.iter()
                .filter_map(|&asset| self.trace_to_ultimate_source(asset, &symbol))
                .collect();

            if ultimate_sources.len() == 1 {
                // All re-export the same thing - OK
                let ultimate = ultimate_sources.into_iter().next().unwrap();
                self.provide_symbol_with_ultimate_source(providers[0], symbol, ultimate);
            } else {
                // Genuine ambiguity - error
                self.record_ambiguous_symbol_error(symbol, providers);
            }
        }
    }
}
```

### 2. Circular Dependencies

**Problem**: Assets that import from each other

**Detection**:

```rust
fn detect_circular_dependency(&mut self, req: &SymbolRequest) -> bool {
    // If the target asset also requests symbols from the requesting asset
    let reverse_requests = self.get_requests_from_to(req.requested_from, req.requesting_asset);
    !reverse_requests.is_empty()
}

fn handle_circular_request(&mut self, req: SymbolRequest) {
    // Add to circular resolution queue - handle after all non-circular requests
    self.circular_requests.push(req);
}

fn resolve_circular_dependencies(&mut self) {
    // Use iterative approach similar to current system
    let mut changed = true;
    while changed {
        changed = false;

        for req in &self.circular_requests.clone() {
            if let Some(provider) = self.try_resolve_circular_request(req) {
                self.fulfill_circular_request(req.clone(), provider);
                changed = true;
            }
        }
    }
}
```

### 3. Transformation Ordering

**Problem**: Dependencies may not be processed yet when encountering imports

**Solution**: The demand-driven approach naturally handles this:

- Record requests for unprocessed dependencies
- Fulfill requests when dependencies are eventually processed
- No ordering constraints required

### 4. Side Effect Analysis

**Important**: Must preserve current side-effect behavior for barrel file elimination

```rust
struct SymbolProviderInfo {
    // ... other fields
    asset_has_side_effects: bool,
    symbol_has_side_effects: bool,
}

fn can_eliminate_barrel_file(&self, barrel_asset: AssetId, symbol: &Symbol) -> bool {
    if let Some(provider) = self.symbol_providers.get(&(barrel_asset, symbol.clone())) {
        // Only eliminate if barrel file is side-effect free (current behavior)
        !provider.asset_has_side_effects && provider.is_reexport
    } else {
        false
    }
}
```

## Integration with Existing System

### Barrel File Elimination

The demand-driven system naturally provides the data needed for barrel file elimination in `BundleGraph.fromAssetGraph()`:

```rust
// Instead of usedSymbolsUp, use resolved symbol information
struct ResolvedDependency {
    dependency_id: DependencyId,
    target_asset: AssetId,
    resolved_symbols: HashMap<Symbol, SymbolProviderInfo>,
}

// In BundleGraph creation:
for resolved_dep in &asset.resolved_dependencies {
    for (symbol, provider) in &resolved_dep.resolved_symbols {
        if provider.is_reexport && !provider.asset_has_side_effects {
            // Create direct dependency to ultimate source
            if let Some((ultimate_asset, ultimate_symbol)) = provider.ultimate_source {
                create_direct_dependency(asset.id, ultimate_asset, ultimate_symbol);
            }
        }
    }
}
```

### Error Reporting

Enhanced error reporting with exact source locations:

```rust
fn generate_symbol_errors(&self) -> Vec<SymbolError> {
    let mut errors = Vec::new();

    // Unresolved symbols
    for (request, contexts) in &self.unresolved_requests {
        for context in contexts {
            errors.push(SymbolError::NotFound {
                symbol: request.symbol.clone(),
                requesting_asset: context.requesting_asset,
                requesting_location: context.source_location,
                target_asset: request.requested_from,
                suggestion: self.suggest_similar_symbols(request),
            });
        }
    }

    // Ambiguous symbols
    for ambiguous_error in &self.ambiguous_symbol_errors {
        errors.push(ambiguous_error.clone());
    }

    errors
}
```

## Performance Benefits

1. **Eliminates Post-Processing**: No separate symbol propagation phase
2. **Incremental Resolution**: Work distributed across asset processing
3. **Early Error Detection**: Symbol errors caught immediately when target assets are processed
4. **Better Cache Locality**: Symbol data processed while AST is hot in memory
5. **Natural Parallelization**: Assets can be transformed in parallel with lock-free symbol resolution

## Implementation Strategy

### Phase 1: Basic Request/Fulfillment

1. Implement core `SymbolTracker` data structures
2. Add symbol request recording during transformation
3. Add symbol provider registration during transformation
4. Basic fulfillment mechanism for direct imports

### Phase 2: Re-export Support

1. Add re-export forwarding logic
2. Implement ultimate source tracking for barrel file elimination
3. Handle `export *` namespace forwarding

### Phase 3: Edge Cases

1. Circular dependency detection and resolution
2. Export \* ambiguity handling
3. Comprehensive error reporting

### Phase 4: Integration

1. Replace existing symbol propagation in asset graph building
2. Update `BundleGraph.fromAssetGraph()` to use new resolution data
3. Performance testing and optimization

## Open Questions

1. **Memory Usage**: How does keeping all symbol requests in memory compare to current approach?
2. **Parallelization**: How to handle shared `SymbolTracker` state across parallel asset transformation?
3. **Caching**: How does this interact with Atlaspack's existing caching system?
4. **Development Experience**: How to provide good debugging tools for this approach?

## Success Criteria

- [ ] Eliminate post-processing symbol propagation step
- [ ] Maintain identical behavior for barrel file elimination
- [ ] Preserve all current error detection and reporting
- [ ] Handle all edge cases (circular deps, export \* ambiguity)
- [ ] Achieve equal or better performance than current system
- [ ] Support incremental builds (future enhancement)
