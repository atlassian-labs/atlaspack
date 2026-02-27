use std::{collections::HashMap, sync::Arc, time::Duration};

use atlaspack_bundling::{Bundler, IdealGraphBundler, ideal_graph::types::IdealGraphBuildOptions};
use atlaspack_core::{
  asset_graph::AssetGraph,
  bundle_graph::native_bundle_graph::NativeBundleGraph,
  types::{
    Asset, Dependency, DependencyBuilder, Environment, FileType, Priority, SpecifierType, Target,
  },
};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rand::{Rng, SeedableRng, prelude::SliceRandom, rngs::StdRng};

/// Parameters controlling the synthetic graph shape.
///
/// The generator creates a modular, cluster-based graph that aims to match the
/// characteristics of a real large-scale application (e.g., Confluence with 60k assets).
///
/// ## Architecture
///
/// - **Layer 0**: Entry assets (4 for xlarge)
/// - **Layer 1**: Route modules — always lazy-loaded from entries, making them bundle roots
/// - **Layer 2**: Component modules — partitioned into cluster-local (80%) and shared (20%)
/// - **Layer 3**: Shared utility modules — imported by components globally
/// - **CSS**: Small number of stylesheets imported by random components
///
/// ## Cluster Structure
///
/// Routes and their components are organized into `num_clusters` modular clusters.
/// Each cluster has its own set of local components, creating isolated subtrees with
/// limited cross-cluster overlap (only through shared components).
///
/// This produces realistic availability density (~15%) instead of the ~65% density
/// that a flat random graph would generate. Real apps like Confluence have ~8.8% density.
///
/// ## Fill-up Loop
///
/// After structured deps, remaining deps are added with cluster-respecting ratios:
/// - 76% intra-cluster component→component sync
/// - 0.5% route→route lazy (limited cross-route connectivity)
/// - 0.5% component→route lazy (creates bundle root graph edges)
/// - 11% component→shared-component sync
/// - 12% component→util sync
///
/// ## Target Metrics (from real Confluence, 60k assets)
///
/// | Metric             | Real App  | Synthetic xlarge |
/// |--------------------|-----------|------------------|
/// | Bundle roots       | 7,261     | ~6,000           |
/// | Availability density | 8.8%    | ~15%             |
/// | Bundling time      | 1.1s      | ~1.9s            |
#[derive(Debug, Clone, Copy)]
struct GraphConfig {
  /// Number of entry points (e.g., 4 for a multi-entry app).
  num_entries: usize,
  /// Total number of assets in the graph.
  num_assets: usize,
  /// Target total number of dependencies (sync + lazy).
  num_deps: usize,
  /// Number of route clusters. Routes and their local components are partitioned
  /// into this many isolated groups. Larger values create smaller, denser clusters;
  /// smaller values create larger, sparser clusters. Tune to achieve ~10-15%
  /// availability density. (large=5, xlarge=15)
  num_clusters: usize,
  /// Fraction of assets that should be CSS (0.0 - 1.0).
  css_ratio: f64,
  /// Fraction of non-entry assets that should be routes/lazy roots (0.0 - 1.0).
  route_ratio: f64,
  /// Fraction of components that should be shared across all routes (0.0 - 1.0).
  shared_component_ratio: f64,
  /// Number of shared components imported by each route.
  ///
  /// This is intentionally a *subset* to avoid an all-to-all explosion in structured deps.
  /// A small "core" subset is imported by all routes to ensure meaningful overlap.
  shared_imports_per_route: usize,
  /// Fraction of deps that should be back-edges creating cycles (0.0 - 1.0).
  cycle_ratio: f64,
  seed: u64,
}

fn kind_tag(kind: &str) -> u8 {
  match kind {
    "entry" => 0x01,
    "route" => 0x02,
    "component" => 0x03,
    "util" => 0x04,
    "styles" => 0x05,
    _ => 0x0f,
  }
}

/// Returns `(asset_id_hex, file_path)`.
///
/// `asset_id_hex` is a 32-char hex string (md5-like) so that `NativeBundleGraph` can derive
/// stable, unique public ids.
fn make_ids(kind: &str, idx: usize, file_type: FileType) -> (String, String) {
  let ext = match file_type {
    FileType::Js => "js",
    FileType::Css => "css",
    _ => "js",
  };

  let file_path = format!("{kind}-{idx}.{ext}");
  // 32 hex chars: 2 (kind tag) + 30 (index). Keep <= 16 bytes so `generate_public_id` doesn't
  // overflow its internal `u128` accumulator.
  let asset_id = format!("{:02x}{:030x}", kind_tag(kind), idx);
  (asset_id, file_path)
}

fn create_asset(id: String, file_path: String, file_type: FileType) -> Asset {
  Asset {
    id,
    file_path: file_path.into(),
    file_type,
    env: Arc::new(Environment::default()),
    is_bundle_splittable: true,
    ..Asset::default()
  }
}

fn add_edge_with_dep(
  asset_graph: &mut AssetGraph,
  asset_nodes: &HashMap<String, usize>,
  from_asset_id: &str,
  to_asset_id: &str,
  specifier: String,
  priority: Priority,
  dep_counter: &mut usize,
) {
  let dep = DependencyBuilder::default()
    .specifier(specifier)
    .specifier_type(SpecifierType::Esm)
    .env(Arc::new(Environment::default()))
    .priority(priority)
    .source_asset_id(from_asset_id.into())
    .build();
  *dep_counter += 1;

  let dep_node = asset_graph.add_dependency(dep, false);
  asset_graph.add_edge(asset_nodes.get(from_asset_id).unwrap(), &dep_node);
  asset_graph.add_edge(&dep_node, asset_nodes.get(to_asset_id).unwrap());
}

/// Generates a realistic synthetic asset graph.
///
/// Deterministic: uses `seed` for RNG.
fn generate_asset_graph(cfg: GraphConfig) -> AssetGraph {
  assert!(cfg.num_entries > 0, "need at least one entry");
  assert!(
    cfg.num_assets >= cfg.num_entries,
    "num_assets must include entries"
  );

  let mut rng = StdRng::seed_from_u64(cfg.seed);
  let mut asset_graph = AssetGraph::new();

  // Decide which assets are CSS.
  //
  // We *avoid* making entries CSS; entries should be JS.
  // We'll allocate CSS mostly in the component layer and some in the utility layer.
  let css_assets_target = ((cfg.num_assets as f64) * cfg.css_ratio).round() as usize;

  // Rough layer sizing. These ratios are tuned so the structure roughly resembles
  // app graphs while still allowing us to hit the total asset/dep counts.
  let num_entries = cfg.num_entries.min(cfg.num_assets);
  let remaining_assets = cfg.num_assets - num_entries;

  // Allocate assets across: routes, components, utilities.
  // Keep utilities relatively small but heavily shared.
  let mut num_routes = ((remaining_assets as f64) * cfg.route_ratio).round() as usize;
  // Components take the bulk of remaining assets after routes.
  let component_ratio = 1.0 - cfg.route_ratio - 0.05; // 5% reserved for utilities
  let mut num_components = ((remaining_assets as f64) * component_ratio).round() as usize;
  let mut num_utils = remaining_assets.saturating_sub(num_routes + num_components);

  // Ensure minimums for structure.
  num_routes = num_routes.clamp(1, remaining_assets.max(1));
  num_utils = num_utils.clamp(1, remaining_assets.max(1));

  // Recompute components from the remainder.
  num_components = remaining_assets.saturating_sub(num_routes + num_utils);
  if num_components == 0 {
    // Prefer having at least 1 component; steal from routes if needed.
    if num_routes > 1 {
      num_routes -= 1;
      num_components = 1;
    } else {
      // If extremely small, steal from utils.
      num_utils = num_utils.saturating_sub(1);
      num_components = 1;
    }
  }

  // --- Create assets and nodes ---
  let mut asset_nodes: HashMap<String, usize> = HashMap::with_capacity(cfg.num_assets);

  // Entries
  let mut entry_asset_ids: Vec<String> = Vec::with_capacity(num_entries);
  let mut entry_file_paths: Vec<String> = Vec::with_capacity(num_entries);
  for i in 0..num_entries {
    let (asset_id, file_path) = make_ids("entry", i, FileType::Js);
    entry_asset_ids.push(asset_id.clone());
    entry_file_paths.push(file_path.clone());

    let target = Target::default();
    // Entry dependency specifier/name should be human readable.
    let entry_dep = Dependency::entry(file_path.clone(), target);
    let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

    let node_id = asset_graph.add_asset(
      Arc::new(create_asset(asset_id.clone(), file_path, FileType::Js)),
      false,
    );
    asset_graph.add_edge(&entry_dep_node, &node_id);
    asset_nodes.insert(asset_id, node_id);
  }

  // Routes (lazy roots)
  let mut route_asset_ids: Vec<String> = Vec::with_capacity(num_routes);
  let mut route_file_paths: Vec<String> = Vec::with_capacity(num_routes);
  for i in 0..num_routes {
    let (asset_id, file_path) = make_ids("route", i, FileType::Js);
    route_asset_ids.push(asset_id.clone());
    route_file_paths.push(file_path.clone());
    let node_id = asset_graph.add_asset(
      Arc::new(create_asset(asset_id.clone(), file_path, FileType::Js)),
      false,
    );
    asset_nodes.insert(asset_id, node_id);
  }

  // Utilities (shared)
  let mut util_asset_ids: Vec<String> = Vec::with_capacity(num_utils);
  let mut util_file_paths: Vec<String> = Vec::with_capacity(num_utils);
  for i in 0..num_utils {
    let (asset_id, file_path) = make_ids("util", i, FileType::Js);
    util_asset_ids.push(asset_id.clone());
    util_file_paths.push(file_path.clone());
    let node_id = asset_graph.add_asset(
      Arc::new(create_asset(asset_id.clone(), file_path, FileType::Js)),
      false,
    );
    asset_nodes.insert(asset_id, node_id);
  }

  // Components (JS) + optional CSS siblings
  //
  // To create type-change boundaries we model CSS as separate assets imported from components.
  let mut component_asset_ids: Vec<String> = Vec::with_capacity(num_components);
  let mut component_file_paths: Vec<String> = Vec::with_capacity(num_components);
  let mut css_asset_ids: Vec<String> = Vec::with_capacity(css_assets_target);
  let mut css_file_paths: Vec<String> = Vec::with_capacity(css_assets_target);

  for i in 0..num_components {
    let (asset_id, file_path) = make_ids("component", i, FileType::Js);
    component_asset_ids.push(asset_id.clone());
    component_file_paths.push(file_path.clone());
    let node_id = asset_graph.add_asset(
      Arc::new(create_asset(asset_id.clone(), file_path, FileType::Js)),
      false,
    );
    asset_nodes.insert(asset_id, node_id);
  }

  // Create CSS assets (separate pool). We'll import them from some components.
  for i in 0..css_assets_target {
    let (asset_id, file_path) = make_ids("styles", i, FileType::Css);
    css_asset_ids.push(asset_id.clone());
    css_file_paths.push(file_path.clone());
    let node_id = asset_graph.add_asset(
      Arc::new(create_asset(asset_id.clone(), file_path, FileType::Css)),
      false,
    );
    asset_nodes.insert(asset_id, node_id);
  }

  // If CSS target exceeded what fits, clamp by actual created.
  // (This can happen for very small graphs where entries already consume the budget.)

  // --- Cluster partitioning ---
  // Partition routes into clusters
  let num_clusters = cfg.num_clusters.max(1);
  let mut cluster_routes: Vec<Vec<String>> = vec![Vec::new(); num_clusters];
  for (i, route_id) in route_asset_ids.iter().enumerate() {
    cluster_routes[i % num_clusters].push(route_id.clone());
  }

  // Split components: 20% shared globally, 80% cluster-local
  let num_shared =
    ((component_asset_ids.len() as f64) * cfg.shared_component_ratio).round() as usize;
  let shared_component_ids: Vec<String> = component_asset_ids[..num_shared].to_vec();
  let local_component_ids: Vec<String> = component_asset_ids[num_shared..].to_vec();
  let mut cluster_components: Vec<Vec<String>> = vec![Vec::new(); num_clusters];
  for (i, comp_id) in local_component_ids.iter().enumerate() {
    cluster_components[i % num_clusters].push(comp_id.clone());
  }

  // --- Create dependencies ---
  let mut dep_counter: usize = 0;
  let mut dep_count: usize = 0;

  // Keep the structured (layered) portion well below the total dep target so we still have
  // budget left for the fill-up loop, which adds random connectivity and a realistic number of
  // lazy deps/bundle boundaries.
  let num_cycle_edges = ((cfg.num_deps as f64) * cfg.cycle_ratio.clamp(0.0, 1.0)).round() as usize;
  let pre_cycle_target_deps = cfg.num_deps.saturating_sub(num_cycle_edges);
  let structured_budget = ((pre_cycle_target_deps as f64) * 0.55).round() as usize;

  // Helper to add an edge and count.
  let add_dep = |asset_graph: &mut AssetGraph,
                 from: &str,
                 to: &str,
                 prio: Priority,
                 spec: String,
                 dep_counter: &mut usize,
                 dep_count: &mut usize| {
    add_edge_with_dep(asset_graph, &asset_nodes, from, to, spec, prio, dep_counter);
    *dep_count += 1;
  };

  // 1) Entry -> routes (mostly lazy)
  // Each entry lazily imports a handful of routes.
  let routes_per_entry = (5usize).min(route_asset_ids.len()).max(1);
  for entry in &entry_asset_ids {
    if dep_count >= structured_budget {
      break;
    }
    let chosen: Vec<_> = route_asset_ids
      .choose_multiple(&mut rng, routes_per_entry)
      .cloned()
      .collect();
    for (j, route) in chosen.iter().enumerate() {
      let prio = Priority::Lazy; // Routes are always lazy-loaded from entries
      add_dep(
        &mut asset_graph,
        entry,
        route,
        prio,
        // Specifier is only used for stable dependency identity; keep it readable.
        format!("./{}?e={j}", &route_file_paths[j % route_file_paths.len()]),
        &mut dep_counter,
        &mut dep_count,
      );
    }
  }

  // 2) Routes -> components (sync)
  //
  // Model route-level code-splitting with clusters: each route imports components from its own
  // cluster, plus a small shared core imported across all routes.

  // Route -> cluster-local components (sync)
  for (cluster_idx, routes) in cluster_routes.iter().enumerate() {
    let local_comps = &cluster_components[cluster_idx];
    if local_comps.is_empty() {
      continue;
    }
    for route_id in routes {
      let num_local_imports = cfg.shared_imports_per_route.min(local_comps.len());
      for comp_id in local_comps.choose_multiple(&mut rng, num_local_imports) {
        if dep_count >= structured_budget {
          break;
        }
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          route_id,
          comp_id,
          Priority::Sync,
          format!("./local-comp?n={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
    }
  }

  // Route -> shared components (sync)
  if !shared_component_ids.is_empty() {
    for route_id in &route_asset_ids {
      let num_shared_imports = (cfg.shared_imports_per_route / 2).min(shared_component_ids.len());
      for comp_id in shared_component_ids.choose_multiple(&mut rng, num_shared_imports) {
        if dep_count >= structured_budget {
          break;
        }
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          route_id,
          comp_id,
          Priority::Sync,
          format!("./shared-comp?n={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
    }
  }

  // --- Shared core utils (imported by shared core components) ---
  //
  // Avoid all-to-all (shared components -> all shared utils). Instead each shared component imports
  // a small overlapping subset.
  let num_shared_utils = (util_asset_ids.len() / 2).max(1);
  let shared_util_ids: Vec<String> = util_asset_ids
    .iter()
    .take(num_shared_utils.min(util_asset_ids.len()))
    .cloned()
    .collect();

  // Ensure overlap by importing a tiny core set from every shared component.
  let core_utils_count = 1usize.min(shared_util_ids.len());
  let (core_shared_util_ids, shared_util_pool) = shared_util_ids.split_at(core_utils_count);

  for shared_comp_id in &shared_component_ids {
    // 1) core util(s)
    for shared_util_id in core_shared_util_ids {
      if dep_count >= structured_budget {
        break;
      }
      let n = dep_count;
      add_dep(
        &mut asset_graph,
        shared_comp_id.as_str(),
        shared_util_id.as_str(),
        Priority::Sync,
        format!("./shared-util?core=1&su={n}"),
        &mut dep_counter,
        &mut dep_count,
      );
    }

    // 2) random subset from the remaining pool
    if dep_count >= structured_budget || shared_util_pool.is_empty() {
      continue;
    }

    let shared_utils_per_component = rng.r#gen_range(3..=5).min(shared_util_ids.len());
    let remaining_needed = shared_utils_per_component.saturating_sub(core_utils_count);
    if remaining_needed == 0 {
      continue;
    }

    let chosen: Vec<_> = shared_util_pool
      .choose_multiple(&mut rng, remaining_needed.min(shared_util_pool.len()))
      .cloned()
      .collect();

    for shared_util_id in chosen {
      if dep_count >= structured_budget {
        break;
      }
      let n = dep_count;
      add_dep(
        &mut asset_graph,
        shared_comp_id.as_str(),
        shared_util_id.as_str(),
        Priority::Sync,
        format!("./shared-util?core=0&su={n}"),
        &mut dep_counter,
        &mut dep_count,
      );
    }
  }

  // 3) Components -> utilities (sync, shared)
  // Many components import a few shared utils.
  //
  // Keep this relatively bounded so that the structured portion doesn't hit cfg.num_deps before the
  // fill-up loop (which adds the remaining edges incl. lazy deps).
  let remaining_structured = structured_budget.saturating_sub(dep_count);
  let utils_per_component = if component_asset_ids.is_empty() {
    1usize
  } else {
    (remaining_structured / component_asset_ids.len()).clamp(1, 3)
  }
  .min(util_asset_ids.len())
  .max(1);

  for component in &component_asset_ids {
    if dep_count >= structured_budget {
      break;
    }

    let chosen: Vec<_> = util_asset_ids
      .choose_multiple(&mut rng, utils_per_component)
      .cloned()
      .collect();
    for (j, util) in chosen.iter().enumerate() {
      if dep_count >= structured_budget {
        break;
      }
      add_dep(
        &mut asset_graph,
        component,
        util,
        Priority::Sync,
        format!("./{}?c={j}", &util_file_paths[j % util_file_paths.len()]),
        &mut dep_counter,
        &mut dep_count,
      );
    }
  }

  // 4) Components -> CSS (sync type-change boundary)
  // Pick a fraction of components to import CSS.
  if !css_asset_ids.is_empty() {
    let css_importing_components = ((component_asset_ids.len() as f64) * 0.10).round() as usize;
    let css_importing_components = css_importing_components.clamp(1, component_asset_ids.len());

    // Each selected component imports 1-2 CSS assets.
    for component in component_asset_ids
      .choose_multiple(&mut rng, css_importing_components)
      .cloned()
      .collect::<Vec<_>>()
    {
      let imports = rng.r#gen_range(1..=2).min(css_asset_ids.len());
      let chosen: Vec<_> = css_asset_ids
        .choose_multiple(&mut rng, imports)
        .cloned()
        .collect();
      for (j, css) in chosen.iter().enumerate() {
        if dep_count >= structured_budget {
          break;
        }
        add_dep(
          &mut asset_graph,
          component.as_str(),
          css.as_str(),
          Priority::Sync,
          format!("./{}?s={j}", &css_file_paths[j % css_file_paths.len()]),
          &mut dep_counter,
          &mut dep_count,
        );
      }
    }
  }

  // At this point we have a realistic shape, but may be below cfg.num_deps.
  // Add additional deps to reach the target count while keeping structure reasonable.
  //
  // Strategy:
  // - Add more component->component sync edges (component tree fan-out)
  // - Add a small number of route->route lazy edges (route-level lazy imports)
  // - Add more component->util edges (shared diamonds)
  let all_js_assets: Vec<String> = entry_asset_ids
    .iter()
    .chain(route_asset_ids.iter())
    .chain(component_asset_ids.iter())
    .chain(util_asset_ids.iter())
    .cloned()
    .collect();

  // (computed above)

  // Fill-up loop (cluster-respecting):
  // - 76%: intra-cluster component->component sync
  // - 0.5%: route->route lazy (limited cross-route connectivity)
  // - 0.5%: component->route lazy (creates bundle root graph edges)
  // - 10%: component->shared-component sync
  // - 10%: component->util sync
  let mut safety = 0;
  while dep_count < pre_cycle_target_deps {
    safety += 1;
    if safety > pre_cycle_target_deps * 10 {
      break;
    }
    let roll: f64 = rng.r#gen();

    if roll < 0.76 {
      // Intra-cluster component -> component sync
      let cluster_idx = rng.r#gen_range(0..num_clusters);
      let cluster_comps = &cluster_components[cluster_idx];
      if cluster_comps.len() >= 2 {
        let from = cluster_comps.choose(&mut rng).unwrap().as_str();
        let to = cluster_comps.choose(&mut rng).unwrap().as_str();
        if from != to {
          let n = dep_count;
          add_dep(
            &mut asset_graph,
            from,
            to,
            Priority::Sync,
            format!("./intra-cluster?n={n}"),
            &mut dep_counter,
            &mut dep_count,
          );
        }
      }
      continue;
    }

    if roll < 0.765 {
      // Route -> route lazy (limited, 0.5% of fill-up)
      if route_asset_ids.len() >= 2 {
        let from = route_asset_ids.choose(&mut rng).unwrap().as_str();
        let to = route_asset_ids.choose(&mut rng).unwrap().as_str();
        if from != to {
          let n = dep_count;
          add_dep(
            &mut asset_graph,
            from,
            to,
            Priority::Lazy,
            format!("./lazy-route?n={n}"),
            &mut dep_counter,
            &mut dep_count,
          );
        }
      }
      continue;
    }

    if roll < 0.77 {
      // Component -> route lazy (0.5% of fill-up, creates bundle root graph edges)
      if !component_asset_ids.is_empty() && !route_asset_ids.is_empty() {
        let from = component_asset_ids.choose(&mut rng).unwrap().as_str();
        let to = route_asset_ids.choose(&mut rng).unwrap().as_str();
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          from,
          to,
          Priority::Lazy,
          format!("./lazy-comp-route?n={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
      continue;
    }

    if roll < 0.88 {
      // Component -> shared-component sync
      if !component_asset_ids.is_empty() && !shared_component_ids.is_empty() {
        let from = component_asset_ids.choose(&mut rng).unwrap().as_str();
        let to = shared_component_ids.choose(&mut rng).unwrap().as_str();
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          from,
          to,
          Priority::Sync,
          format!("./shared?n={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
      continue;
    }

    // Component -> util sync
    if !component_asset_ids.is_empty() && !util_asset_ids.is_empty() {
      let from = component_asset_ids.choose(&mut rng).unwrap().as_str();
      let to = util_asset_ids.choose(&mut rng).unwrap().as_str();
      let n = dep_count;
      add_dep(
        &mut asset_graph,
        from,
        to,
        Priority::Sync,
        format!("./util?n={n}"),
        &mut dep_counter,
        &mut dep_count,
      );
    }
  }

  // --- Add back-edges to create cycles (realistic apps have cycles) ---
  // Prefer targeting earlier assets to increase the probability of producing a cycle.
  for _ in 0..num_cycle_edges {
    if dep_count >= cfg.num_deps {
      break;
    }

    if all_js_assets.len() >= 2 {
      // Prefer component -> route or util -> component back-edges.
      let from_idx = rng.r#gen_range(0..all_js_assets.len());
      let to_idx = rng.r#gen_range(0..from_idx.max(1));
      let from = &all_js_assets[from_idx];
      let to = &all_js_assets[to_idx];

      if from != to {
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          from,
          to,
          Priority::Sync,
          format!("./{to}?cycle={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
    }
  }

  asset_graph
}

fn apply_group_tuning(
  group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
  name: &str,
) {
  match name {
    "large" => {
      group.sample_size(15);
      group.measurement_time(Duration::from_secs(30));
    }
    "xlarge" => {
      group.sample_size(10);
      group.measurement_time(Duration::from_secs(60));
    }
    _ => {}
  }
}

fn benchmark_ideal_graph(c: &mut Criterion) {
  let mut group = c.benchmark_group("ideal_graph");

  let configs = [
    ("large", 4, 10_000, 37_100, 0.10, 5),
    ("xlarge", 4, 60_000, 222_600, 0.10, 15),
  ];

  for (name, entries, assets, deps, route_ratio, num_clusters) in configs {
    apply_group_tuning(&mut group, name);

    let graph = generate_asset_graph(GraphConfig {
      num_entries: entries,
      num_assets: assets,
      num_deps: deps,
      num_clusters,
      css_ratio: 0.001,
      route_ratio,
      shared_component_ratio: 0.20,
      shared_imports_per_route: 15,
      cycle_ratio: 0.02,
      seed: 42,
    });

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());

    group.bench_function(BenchmarkId::new("build", name), |b| {
      b.iter(|| {
        let (ideal, stats, report) = bundler.build_ideal_graph(black_box(&graph)).unwrap();
        black_box((ideal, stats, report));
      })
    });
  }

  group.finish();
}

fn benchmark_full_bundle(c: &mut Criterion) {
  let mut group = c.benchmark_group("full_bundle");

  let configs = [
    ("large", 4, 10_000, 37_100, 0.10, 5),
    ("xlarge", 4, 60_000, 222_600, 0.10, 15),
  ];

  for (name, entries, assets, deps, route_ratio, num_clusters) in configs {
    apply_group_tuning(&mut group, name);

    let graph = generate_asset_graph(GraphConfig {
      num_entries: entries,
      num_assets: assets,
      num_deps: deps,
      num_clusters,
      css_ratio: 0.001,
      route_ratio,
      shared_component_ratio: 0.20,
      shared_imports_per_route: 15,
      cycle_ratio: 0.02,
      seed: 42,
    });

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());

    group.bench_function(BenchmarkId::new("bundle", name), |b| {
      b.iter(|| {
        let mut bundle_graph = NativeBundleGraph::from_asset_graph(black_box(&graph));
        bundler
          .bundle(black_box(&graph), black_box(&mut bundle_graph))
          .unwrap();
        black_box(bundle_graph);
      })
    });
  }

  group.finish();
}

criterion_group!(benches, benchmark_ideal_graph, benchmark_full_bundle);
criterion_main!(benches);
