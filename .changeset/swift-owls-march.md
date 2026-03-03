---
'@atlaspack/rust': patch
---

Replace `propagate_requested_symbols` with SymbolTracker-based propagation in native symbol propagation.

The SymbolTracker now handles both symbol tracking and propagation in a single unified flow via
`track_symbols()`, which returns dependency IDs that need un-deferral. This replaces the separate
`propagate_requested_symbols` function when the `rustSymbolTracker` feature flag is enabled.

Key changes:

- `track_symbols()` now returns `Vec<DependencyId>` of dependencies needing un-deferral
- New `propagate_to_outgoing_dependencies()` method determines which deps need un-deferral
- New `has_requested_symbols()` method replaces graph-based deferral checks
- Extracted `track_dependency_symbols()` for cleaner separation of concerns
- Replaced `is_star_reexport_symbol`/`is_namespace_reexport_symbol` with `classify_symbol_export()` enum
- Feature-flagged integration in `asset_graph_request.rs` with `track_and_propagate_symbols()` helper
