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
/// The generator is intentionally simple and layered, but aims to resemble a real application:
///
/// - Layer 0: entry assets
/// - Layer 1: route modules (lazy deps from entries)
/// - Layer 2: component modules (sync deps from routes)
/// - Layer 3: shared utility modules (sync deps from many components)
/// - Plus: some components import CSS (sync deps with a type-change boundary)
///
/// We then add extra sync deps to reach the requested total dep count.
#[derive(Debug, Clone, Copy)]
struct GraphConfig {
  num_entries: usize,
  num_assets: usize,
  num_deps: usize,
  /// Fraction of deps that should be lazy (0.0 - 1.0).
  lazy_ratio: f64,
  /// Fraction of assets that should be CSS (0.0 - 1.0).
  css_ratio: f64,
  /// Fraction of non-entry assets that should be routes/lazy roots (0.0 - 1.0).
  route_ratio: f64,
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
  let component_ratio = 1.0 - cfg.route_ratio - 0.15; // 15% reserved for utilities
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

  // --- Create dependencies ---
  let mut dep_counter: usize = 0;
  let mut dep_count: usize = 0;

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
    let chosen: Vec<_> = route_asset_ids
      .choose_multiple(&mut rng, routes_per_entry)
      .cloned()
      .collect();
    for (j, route) in chosen.iter().enumerate() {
      let is_lazy = rng.gen_bool(cfg.lazy_ratio.clamp(0.0, 1.0));
      let prio = if is_lazy {
        Priority::Lazy
      } else {
        Priority::Sync
      };
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
  let components_per_route = (20usize).min(component_asset_ids.len()).max(1);
  for route in &route_asset_ids {
    let chosen: Vec<_> = component_asset_ids
      .choose_multiple(&mut rng, components_per_route)
      .cloned()
      .collect();
    for (j, component) in chosen.iter().enumerate() {
      add_dep(
        &mut asset_graph,
        route,
        component,
        Priority::Sync,
        format!(
          "./{}?r={j}",
          &component_file_paths[j % component_file_paths.len()]
        ),
        &mut dep_counter,
        &mut dep_count,
      );
    }
  }

  // 3) Components -> utilities (sync, shared)
  // Many components import a few shared utils.
  let utils_per_component = (3usize).min(util_asset_ids.len()).max(1);
  for component in &component_asset_ids {
    let chosen: Vec<_> = util_asset_ids
      .choose_multiple(&mut rng, utils_per_component)
      .cloned()
      .collect();
    for (j, util) in chosen.iter().enumerate() {
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
      let imports = rng.gen_range(1..=2).min(css_asset_ids.len());
      let chosen: Vec<_> = css_asset_ids
        .choose_multiple(&mut rng, imports)
        .cloned()
        .collect();
      for (j, css) in chosen.iter().enumerate() {
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

  let mut safety = 0usize;
  while dep_count < cfg.num_deps && safety < cfg.num_deps.saturating_mul(2) {
    safety += 1;

    // Choose a pattern type.
    let roll: f64 = rng.r#gen::<f64>();

    if roll < 0.60 && component_asset_ids.len() >= 2 {
      // component -> component sync
      let from = component_asset_ids.choose(&mut rng).unwrap().as_str();
      let to = component_asset_ids.choose(&mut rng).unwrap().as_str();
      if from != to {
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          from,
          to,
          Priority::Sync,
          format!("./component?cc={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
      continue;
    }

    if roll < 0.75 && !route_asset_ids.is_empty() {
      // entry/route -> route lazy
      let from = if rng.gen_bool(0.5) {
        entry_asset_ids.choose(&mut rng).unwrap().as_str()
      } else {
        route_asset_ids.choose(&mut rng).unwrap().as_str()
      };
      let to = route_asset_ids.choose(&mut rng).unwrap().as_str();
      if from != to {
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          from,
          to,
          Priority::Lazy,
          format!("./route?lazy={n}"),
          &mut dep_counter,
          &mut dep_count,
        );
      }
      continue;
    }

    if roll < 0.95 && !util_asset_ids.is_empty() {
      // component -> util sync
      let from = component_asset_ids.choose(&mut rng).unwrap().as_str();
      let to = util_asset_ids.choose(&mut rng).unwrap().as_str();
      let n = dep_count;
      add_dep(
        &mut asset_graph,
        from,
        to,
        Priority::Sync,
        format!("./util?cu={n}"),
        &mut dep_counter,
        &mut dep_count,
      );
      continue;
    }

    // Fallback: random sync edge within JS assets.
    if all_js_assets.len() >= 2 {
      let from = all_js_assets.choose(&mut rng).unwrap().as_str();
      let to = all_js_assets.choose(&mut rng).unwrap().as_str();
      if from != to {
        let n = dep_count;
        add_dep(
          &mut asset_graph,
          from,
          to,
          Priority::Sync,
          format!("./{to}?x={n}"),
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
    "small" => {
      // defaults are fine
    }
    "medium" => {
      group.measurement_time(Duration::from_secs(10));
    }
    "large" => {
      group.sample_size(15);
      group.measurement_time(Duration::from_secs(30));
    }
    "xlarge" => {
      group.sample_size(10);
      group.measurement_time(Duration::from_secs(15));
    }
    _ => {}
  }
}

fn benchmark_ideal_graph(c: &mut Criterion) {
  let mut group = c.benchmark_group("ideal_graph");

  let configs = [
    ("small", 2, 100, 700, 0.05),
    ("medium", 5, 1_000, 7_000, 0.05),
    ("large", 10, 10_000, 70_000, 0.05),
    ("xlarge", 20, 120_000, 900_000, 0.05),
  ];

  for (name, entries, assets, deps, route_ratio) in configs {
    apply_group_tuning(&mut group, name);

    let graph = generate_asset_graph(GraphConfig {
      num_entries: entries,
      num_assets: assets,
      num_deps: deps,
      lazy_ratio: 0.15,
      css_ratio: 0.10,
      route_ratio,
      seed: 42,
    });

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());

    group.bench_function(BenchmarkId::new("build", name), |b| {
      b.iter(|| {
        let (ideal, stats) = bundler.build_ideal_graph(black_box(&graph)).unwrap();
        black_box((ideal, stats));
      })
    });
  }

  group.finish();
}

fn benchmark_full_bundle(c: &mut Criterion) {
  let mut group = c.benchmark_group("full_bundle");

  let configs = [
    ("small", 2, 100, 700, 0.05),
    ("medium", 5, 1_000, 7_000, 0.05),
    ("large", 10, 10_000, 70_000, 0.05),
    ("xlarge", 20, 120_000, 900_000, 0.05),
  ];

  for (name, entries, assets, deps, route_ratio) in configs {
    apply_group_tuning(&mut group, name);

    let graph = generate_asset_graph(GraphConfig {
      num_entries: entries,
      num_assets: assets,
      num_deps: deps,
      lazy_ratio: 0.15,
      css_ratio: 0.10,
      route_ratio,
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
