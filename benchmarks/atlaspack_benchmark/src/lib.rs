use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use petgraph::dot::{Config as DotConfig, Dot};
use petgraph::graph::DiGraph;
use rand::{distributions::Alphanumeric, rngs::StdRng, Rng, SeedableRng};
use rayon::prelude::*;
use tracing::{info, trace};

pub struct GenerateMonorepoParams<'a> {
  pub target_dir: &'a Path,
  pub num_files: usize,
  pub avg_lines_per_file: usize,
  pub depth: usize,
  pub avg_out_degree: f64,
  pub seed: Option<u64>,
  pub dot_output: Option<PathBuf>,
  pub layer_weights: Option<Vec<f64>>,
  pub geo_p: Option<f64>,
  pub subtrees: usize,
  pub cross_edge_prob: f64,
  pub cluster_weights: Option<Vec<f64>>,
  pub async_import_ratio: f64,
}

impl Default for GenerateMonorepoParams<'static> {
  fn default() -> Self {
    Self {
      target_dir: Path::new("target/monorepo"),
      num_files: 1000,
      avg_lines_per_file: 10,
      depth: 5,
      avg_out_degree: 2.0,
      seed: None,
      dot_output: None,
      layer_weights: None,
      geo_p: None,
      subtrees: 4,
      cross_edge_prob: 0.05,
      cluster_weights: None,
      async_import_ratio: 0.001,
    }
  }
}

pub fn generate_monorepo(
  GenerateMonorepoParams {
    target_dir,
    num_files,
    avg_lines_per_file,
    depth,
    avg_out_degree,
    seed,
    dot_output,
    layer_weights,
    geo_p,
    subtrees,
    cross_edge_prob,
    cluster_weights,
    async_import_ratio,
  }: GenerateMonorepoParams,
) -> anyhow::Result<()> {
  info!("creating root structure");
  let pkg_dir = target_dir.join("app-root");
  fs::create_dir_all(&pkg_dir)?;

  info!("writing root files");
  write_json(
    &target_dir.join("package.json"),
    serde_json::json!({
      "name": "generated-app",
      "private": true,
      "main": "app-root/dist/index.js",
      "scripts": {
        "build": "tsc -p app-root",
        "test": "node app-root/dist/index.js"
      }
    }),
  )?;
  // Write an empty yarn.lock at the project root
  write_file(&target_dir.join("yarn.lock"), b"")?;
  // tsconfig for single package
  write_json(
    &pkg_dir.join("tsconfig.json"),
    serde_json::json!({
      "compilerOptions": {
        "target": "ES2020",
        "module": "ES2020",
        "moduleResolution": "Bundler",
        "declaration": true,
        "outDir": "dist",
        "rootDir": "src",
        "strict": true,
        "skipLibCheck": true
      },
      "include": ["src"]
    }),
  )?;

  // src directory and files
  let src_dir = pkg_dir.join("src");
  fs::create_dir_all(&src_dir)?;

  let mut rng: StdRng = match seed {
    Some(s) => StdRng::seed_from_u64(s),
    None => StdRng::from_rng(rand::thread_rng())?,
  };

  info!("building DAG");
  // Build a DAG with a synthetic root entry that reaches the first layer
  let graph = build_dag(BuildDagParams {
    num_nodes: num_files,
    max_depth: depth.max(1),
    avg_out_degree,
    rng: &mut rng,
    layer_weights: layer_weights.as_deref(),
    geo_p,
    subtrees: subtrees.max(1),
    cross_edge_prob,
    cluster_weights: cluster_weights.as_deref(),
    async_import_ratio,
  });
  info!(nodes = num_files, depth = graph.layers.len(), "graph built");

  // Always render DOT via graphviz to a PNG file
  {
    let mut g: DiGraph<(), ()> = DiGraph::new();
    let mut node_ix = Vec::with_capacity(num_files);
    for _ in 0..num_files {
      node_ix.push(g.add_node(()));
    }
    for (src, deps) in graph.adjacency.iter().enumerate() {
      for &dst in deps {
        g.add_edge(node_ix[src], node_ix[dst], ());
      }
    }
    // Add synthetic root that connects to all roots
    let root = g.add_node(());
    let mut indeg = vec![0usize; num_files];
    for (_, deps) in graph.adjacency.iter().enumerate() {
      for &d in deps {
        indeg[d] += 1;
      }
    }
    for n in 0..num_files {
      if indeg[n] == 0 {
        g.add_edge(root, node_ix[n], ());
      }
    }
    let dot = format!("{:?}", Dot::with_config(&g, &[DotConfig::EdgeNoLabel]));
    if let Some(out_path) = dot_output {
      info!(path = %out_path.display(), "rendering DOT to PNG via graphviz");
      let mut child = Command::new("dot")
        .args(["-Tpng", "-o"])
        .arg(&out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn graphviz 'dot'")?;
      {
        let stdin = child.stdin.as_mut().expect("stdin should be piped");
        stdin.write_all(dot.as_bytes())?;
      }
      let status = child.wait()?;
      if !status.success() {
        return Err(anyhow::anyhow!(
          "graphviz 'dot' failed with status {:?}",
          status
        ));
      }
    }
  }

  // Add a synthetic root that imports all layer-0 nodes
  let mut indegree: Vec<usize> = vec![0; num_files];
  for deps in &graph.adjacency {
    for &t in deps {
      indegree[t] += 1;
    }
  }
  let roots: Vec<usize> = (0..num_files).filter(|&n| indegree[n] == 0).collect();

  info!("writing files to disk");
  let range = (0..num_files).collect::<Vec<_>>();
  let chunks = range.chunks(1000);
  for chunk in chunks {
    let results = chunk
      .iter()
      .cloned()
      .map(|node| {
        let file_path = src_dir.join(format!("file_{}.ts", node + 1));
        let content = generate_ts_file_with_imports(
          node,
          &graph.adjacency[node],
          &graph.async_edge[node],
          avg_lines_per_file,
        );
        (file_path, content)
      })
      .collect::<Vec<_>>()
      .into_par_iter()
      .map(|(file_path, content)| -> anyhow::Result<()> {
        write_file(&file_path, content.as_bytes())?;
        Ok(())
      })
      .collect::<Vec<_>>();

    for result in results {
      result?;
    }
  }

  // index.ts: synthetic root that reaches all root nodes
  let mut index_content = String::new();
  for &r in &roots {
    index_content.push_str(&format!(
      "import {{ symbol_{} }} from './file_{}';\n",
      r + 1,
      r + 1
    ));
  }
  index_content.push_str("\nexport async function run() {\n  let acc = 0;\n");
  for &r in &roots {
    index_content.push_str(&format!("  acc += await symbol_{}();\n", r + 1));
  }
  index_content.push_str("  return acc;\n}\n");
  write_file(&src_dir.join("index.ts"), index_content.as_bytes())?;

  Ok(())
}

fn generate_ts_file_with_imports(
  node: usize,
  deps: &[usize],
  async_flags: &[bool],
  avg_lines: usize,
) -> String {
  let lines = avg_lines.max(1);
  let mut s = String::new();
  s.push_str("// autogenerated file\n");
  // Eager static imports for sync edges only; async edges will be loaded dynamically inside the function
  for (idx, &d) in deps.iter().enumerate() {
    if !async_flags.get(idx).copied().unwrap_or(false) {
      s.push_str(&format!(
        "import {{ symbol_{} }} from './file_{}';\n",
        d + 1,
        d + 1
      ));
    }
  }
  s.push('\n');
  for _ in 0..lines {
    let rand_ident: String = rand::thread_rng()
      .sample_iter(&Alphanumeric)
      .take(12)
      .map(char::from)
      .collect();
    s.push_str(&format!(
      "export const v_{}: {{n: number, s: string}} = {{n: Math.random(), s: '{}'}};\n",
      rand_ident, rand_ident
    ));
  }
  s.push('\n');
  s.push_str(&format!(
    "export async function symbol_{}(): Promise<number> {{\n  let acc = 0;\n",
    node + 1
  ));
  for (idx, &d) in deps.iter().enumerate() {
    if async_flags.get(idx).copied().unwrap_or(false) {
      s.push_str(&format!(
        "  acc += (await import('./file_{}')).symbol_{}();\n",
        d + 1,
        d + 1
      ));
    } else {
      s.push_str(&format!("  acc += symbol_{}();\n", d + 1));
    }
  }
  s.push_str(&format!("  return acc + {};\n}}\n", node + 1));
  s
}

struct Graph {
  adjacency: Vec<Vec<usize>>, // edges: node -> dependencies (imports)
  layers: Vec<Vec<usize>>,    // layered nodes for depth control
  async_edge: Vec<Vec<bool>>, // per-edge async flag aligned with adjacency
}

struct BuildDagParams<'a> {
  num_nodes: usize,
  max_depth: usize,
  avg_out_degree: f64,
  rng: &'a mut StdRng,
  layer_weights: Option<&'a [f64]>,
  geo_p: Option<f64>,
  subtrees: usize,
  cross_edge_prob: f64,
  cluster_weights: Option<&'a [f64]>,
  async_import_ratio: f64,
}

fn build_dag(
  BuildDagParams {
    num_nodes,
    max_depth,
    avg_out_degree,
    rng,
    layer_weights,
    geo_p,
    subtrees,
    cross_edge_prob,
    cluster_weights,
    async_import_ratio,
  }: BuildDagParams,
) -> Graph {
  let mut depth = max_depth.max(1).min(num_nodes.max(1));
  // Special-case semantics per request
  if depth <= 1 {
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); 1];
    for n in 0..num_nodes {
      layers[0].push(n);
    }
    let adjacency: Vec<Vec<usize>> = vec![Vec::new(); num_nodes];
    let async_edge: Vec<Vec<bool>> = vec![Vec::new(); num_nodes];
    return Graph {
      adjacency,
      layers,
      async_edge,
    };
  }
  if depth >= num_nodes {
    depth = num_nodes;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); depth];
    for n in 0..num_nodes {
      layers[n].push(n);
    }
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); num_nodes];
    let mut async_edge: Vec<Vec<bool>> = vec![Vec::new(); num_nodes];
    for n in 0..num_nodes.saturating_sub(1) {
      adjacency[n].push(n + 1);
      async_edge[n].push(false);
    }
    return Graph {
      adjacency,
      layers,
      async_edge,
    };
  }

  let mut layers: Vec<Vec<usize>> = vec![Vec::new(); depth];

  // Build a probability distribution over layers
  let probs: Vec<f64> = if let Some(weights) = layer_weights {
    let mut w = weights.to_vec();
    if w.len() < depth {
      w.resize(depth, 0.0);
    }
    let sum: f64 = w.iter().copied().sum();
    let norm = if sum > 0.0 { sum } else { 1.0 };
    w.into_iter().map(|x| x / norm).collect()
  } else if let Some(p) = geo_p {
    // geometric-like decreasing weights across depth
    let p = if p <= 0.0 {
      0.5
    } else if p > 1.0 {
      1.0
    } else {
      p
    };
    let mut w = Vec::with_capacity(depth);
    for i in 0..depth {
      w.push((1.0 - p).powi(i as i32) * p);
    }
    let sum: f64 = w.iter().sum();
    w.into_iter().map(|x| x / sum).collect()
  } else {
    // default: more weight near the top, linearly decreasing
    let mut w = Vec::with_capacity(depth);
    for i in 0..depth {
      w.push((depth - i) as f64);
    }
    let sum: f64 = w.iter().sum();
    w.into_iter().map(|x| x / sum).collect()
  };

  // Sample layer for each node based on probs
  let mut cumulative = Vec::with_capacity(depth);
  let mut acc = 0.0;
  for p in &probs {
    acc += *p;
    cumulative.push(acc);
  }
  for n in 0..num_nodes {
    let r: f64 = rng.gen();
    let mut layer = 0usize;
    while layer + 1 < depth && r > cumulative[layer] {
      layer += 1;
    }
    layers[layer].push(n);
  }
  // Assign nodes to subtrees/clusters
  let subtree_probs: Vec<f64> = if let Some(cw) = cluster_weights {
    let mut w = cw.to_vec();
    if w.len() < subtrees {
      w.resize(subtrees, 0.0);
    }
    let sum: f64 = w.iter().sum();
    let norm = if sum > 0.0 { sum } else { 1.0 };
    w.into_iter().map(|x| x / norm).collect()
  } else {
    vec![1.0 / subtrees as f64; subtrees]
  };
  let mut subtree_cum = Vec::with_capacity(subtrees);
  let mut accp = 0.0;
  for p in &subtree_probs {
    accp += *p;
    subtree_cum.push(accp);
  }
  let mut node_subtree: Vec<usize> = vec![0; num_nodes];
  for n in 0..num_nodes {
    let r: f64 = rng.gen();
    let mut sidx = 0usize;
    while sidx + 1 < subtrees && r > subtree_cum[sidx] {
      sidx += 1;
    }
    node_subtree[n] = sidx;
  }

  // Build edges from each layer to deeper layers with cross-subtree limits
  let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); num_nodes];
  let mut async_edge: Vec<Vec<bool>> = vec![Vec::new(); num_nodes];
  let mut all_deeper: Vec<Vec<usize>> = Vec::with_capacity(depth);
  for d in 0..depth {
    // precompute suffix unions
    let mut v = Vec::new();
    for dd in (d + 1)..depth {
      v.extend_from_slice(&layers[dd]);
    }
    all_deeper.push(v);
  }

  // No explicit backbone chain; let subtrees and probabilities shape the graph

  for d in 0..depth.saturating_sub(1) {
    for &node in &layers[d] {
      let candidates = &all_deeper[d];
      if candidates.is_empty() {
        continue;
      }
      // choose degree around avg_out_degree with light noise
      let noise: f64 = rng.gen_range(-0.5..0.5);
      let mut deg: i32 = (avg_out_degree + noise).floor() as i32;
      if deg < 0 {
        deg = 0;
      }

      // Single-pass classify availability and record simple fallbacks without allocating vectors.
      let src_sub = node_subtree[node];
      let mut has_same = false;
      let mut has_cross = false;
      let mut first_same: Option<usize> = None;
      let mut first_cross: Option<usize> = None;
      for &t in candidates {
        if node_subtree[t] == src_sub {
          if !has_same {
            has_same = true;
            first_same = Some(t);
          }
        } else if !has_cross {
          has_cross = true;
          first_cross = Some(t);
        }
        if has_same && has_cross {
          break;
        }
      }

      let tries = deg as usize;
      let mut chosen: Vec<usize> = Vec::with_capacity(tries);
      let pick_prob = cross_edge_prob.clamp(0.0, 1.0);
      let max_attempts_per_pick = candidates.len().min(16).max(1);

      for _ in 0..tries {
        // decide whether to try cross-subtree or same-subtree target
        let pick_cross = has_cross && (!has_same || rng.gen_bool(pick_prob));
        // sample a target without materializing partitions
        let mut picked: Option<usize> = None;
        for _ in 0..max_attempts_per_pick {
          let t = candidates[rng.gen_range(0..candidates.len())];
          let is_cross = node_subtree[t] != src_sub;
          if pick_cross == is_cross && !chosen.contains(&t) {
            picked = Some(t);
            break;
          }
        }
        // Fallback to the first available candidate in the requested category
        if picked.is_none() {
          if pick_cross {
            if let Some(t) = first_cross {
              if !chosen.contains(&t) {
                picked = Some(t);
              }
            }
          } else if let Some(t) = first_same {
            if !chosen.contains(&t) {
              picked = Some(t);
            }
          }
        }
        // If still none, try the other category as ultimate fallback
        if picked.is_none() {
          if let Some(t) = first_same.or(first_cross) {
            if !chosen.contains(&t) {
              picked = Some(t);
            }
          }
        }

        if let Some(t) = picked {
          chosen.push(t);
          adjacency[node].push(t);
          async_edge[node].push(false);
        }
      }

      // ensure at least one forward edge if none selected and candidates exist
      if adjacency[node].is_empty() {
        if let Some(t) = first_same.or(first_cross) {
          adjacency[node].push(t);
          async_edge[node].push(false);
        }
      }
    }
  }
  // Ensure every non-root node has at least one incoming edge
  let mut indegree: Vec<usize> = vec![0; num_nodes];
  for deps in &adjacency {
    for &t in deps {
      indegree[t] += 1;
    }
  }
  for d in 1..depth {
    // skip first layer
    for &node in &layers[d] {
      if indegree[node] == 0 {
        // connect from earlier layers preferring same subtree
        let mut same_src: Vec<usize> = Vec::new();
        let mut any_src: Vec<usize> = Vec::new();
        for dd in 0..d {
          for &s in &layers[dd] {
            any_src.push(s);
            if node_subtree[s] == node_subtree[node] {
              same_src.push(s);
            }
          }
        }
        let pick_src = if !same_src.is_empty() {
          &same_src
        } else {
          &any_src
        };
        if !pick_src.is_empty() {
          let &src = pick_src.get(rng.gen_range(0..pick_src.len())).unwrap();
          adjacency[src].push(node);
          async_edge[src].push(false);
          indegree[node] = 1;
        }
      }
    }
  }
  // Mark a fraction of edges as async
  let ratio = async_import_ratio.clamp(0.0, 1.0);
  if ratio > 0.0 {
    for s in 0..num_nodes {
      for i in 0..adjacency[s].len() {
        if rng.gen::<f64>() < ratio {
          async_edge[s][i] = true;
        }
      }
    }
  }
  Graph {
    adjacency,
    layers,
    async_edge,
  }
}

fn write_file(path: &Path, content: &[u8]) -> anyhow::Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let mut f = fs::File::create(path)?;
  f.write_all(content)?;
  Ok(())
}

fn write_json(path: &Path, value: serde_json::Value) -> anyhow::Result<()> {
  let s = serde_json::to_string_pretty(&value)?;
  write_file(path, s.as_bytes())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::io::Write;
  use std::path::PathBuf;
  use std::process::{Command, Stdio};

  use petgraph::dot::{Config as DotConfig, Dot};
  use petgraph::graph::DiGraph;
  use rand::SeedableRng;

  fn build_petgraph(graph: &Graph) -> (DiGraph<(), ()>, Vec<petgraph::prelude::NodeIndex>) {
    let num_nodes = graph.adjacency.len();
    let mut g: DiGraph<(), ()> = DiGraph::new();
    let mut node_ix = Vec::with_capacity(num_nodes);
    for _ in 0..num_nodes {
      node_ix.push(g.add_node(()));
    }
    for (src, deps) in graph.adjacency.iter().enumerate() {
      for &dst in deps {
        g.add_edge(node_ix[src], node_ix[dst], ());
      }
    }
    (g, node_ix)
  }

  fn dot_string_with_synthetic_root(graph: &Graph) -> String {
    let (mut g, node_ix) = build_petgraph(graph);
    let num_nodes = node_ix.len();
    let mut indeg = vec![0usize; num_nodes];
    for deps in &graph.adjacency {
      for &d in deps {
        indeg[d] += 1;
      }
    }
    let root = g.add_node(());
    for n in 0..num_nodes {
      if indeg[n] == 0 {
        g.add_edge(root, node_ix[n], ());
      }
    }
    format!("{:?}", Dot::with_config(&g, &[DotConfig::EdgeNoLabel]))
  }

  fn write_svg(name: &str, graph: &Graph) {
    let out_dir = PathBuf::from("target/graph-tests");
    let _ = fs::create_dir_all(&out_dir);
    let dot = dot_string_with_synthetic_root(graph);
    let dot_path = out_dir.join(format!("{}.dot", name));
    let svg_path = out_dir.join(format!("{}.svg", name));
    if let Ok(mut f) = fs::File::create(&dot_path) {
      let _ = f.write_all(dot.as_bytes());
    }
    // Try to render with graphviz if available, but don't fail the test if not present
    if let Ok(mut child) = Command::new("dot")
      .args(["-Tsvg", "-o"]) // N.B. do not add newline here
      .arg(&svg_path)
      .stdin(Stdio::piped())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
    {
      if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(dot.as_bytes());
      }
      let _ = child.wait();
    }
  }

  fn assert_is_dag(graph: &Graph) {
    let num_nodes = graph.adjacency.len();
    let mut indeg = vec![0usize; num_nodes];
    for deps in &graph.adjacency {
      for &t in deps {
        indeg[t] += 1;
      }
    }
    let mut queue: std::collections::VecDeque<usize> =
      (0..num_nodes).filter(|&n| indeg[n] == 0).collect();
    let mut visited = 0usize;
    while let Some(u) = queue.pop_front() {
      visited += 1;
      for &v in &graph.adjacency[u] {
        indeg[v] -= 1;
        if indeg[v] == 0 {
          queue.push_back(v);
        }
      }
    }
    assert_eq!(visited, num_nodes, "graph contains a cycle");
  }

  fn layer_index_map(layers: &[Vec<usize>], num_nodes: usize) -> Vec<usize> {
    let mut idx = vec![usize::MAX; num_nodes];
    for (d, layer) in layers.iter().enumerate() {
      for &n in layer {
        idx[n] = d;
      }
    }
    idx
  }

  #[test]
  fn build_dag_depth_one_has_no_edges_and_single_layer() {
    let num_nodes = 10;
    let mut rng = StdRng::seed_from_u64(42);
    let g = build_dag(BuildDagParams {
      num_nodes,
      max_depth: 1,
      avg_out_degree: 2.0,
      rng: &mut rng,
      layer_weights: None,
      geo_p: None,
      subtrees: 3,
      cross_edge_prob: 0.1,
      cluster_weights: None,
      async_import_ratio: 0.0,
    });

    assert_eq!(g.layers.len(), 1);
    assert_eq!(g.layers[0].len(), num_nodes);
    assert_eq!(g.adjacency.len(), num_nodes);
    assert_eq!(g.async_edge.len(), num_nodes);
    for n in 0..num_nodes {
      assert!(g.adjacency[n].is_empty());
      assert!(g.async_edge[n].is_empty());
    }

    assert_is_dag(&g);
  }

  #[test]
  fn build_dag_depth_ge_nodes_forms_chain_across_layers() {
    let num_nodes = 7;
    let mut rng = StdRng::seed_from_u64(7);
    let g = build_dag(BuildDagParams {
      num_nodes,
      max_depth: num_nodes + 5, // triggers special-case chain
      avg_out_degree: 3.0,
      rng: &mut rng,
      layer_weights: None,
      geo_p: None,
      subtrees: 2,
      cross_edge_prob: 0.5,
      cluster_weights: None,
      async_import_ratio: 0.0,
    });

    assert_eq!(g.layers.len(), num_nodes);
    for i in 0..num_nodes {
      assert_eq!(g.layers[i], vec![i]);
    }
    for i in 0..num_nodes {
      if i + 1 < num_nodes {
        assert_eq!(g.adjacency[i], vec![i + 1]);
        assert_eq!(g.async_edge[i], vec![false]);
      } else {
        assert!(g.adjacency[i].is_empty());
        assert!(g.async_edge[i].is_empty());
      }
    }

    assert_is_dag(&g);
  }

  #[test]
  fn build_dag_general_is_acyclic_edges_forward_and_incoming_edges_present() {
    let num_nodes = 200;
    let depth = 6;
    let mut rng = StdRng::seed_from_u64(12345);
    let g = build_dag(BuildDagParams {
      num_nodes,
      max_depth: depth,
      avg_out_degree: 2.4,
      rng: &mut rng,
      layer_weights: None,
      geo_p: Some(0.3),
      subtrees: 4,
      cross_edge_prob: 0.25,
      cluster_weights: None,
      async_import_ratio: 0.0,
    });

    // basic structure
    assert_eq!(g.adjacency.len(), num_nodes);
    assert_eq!(g.async_edge.len(), num_nodes);
    let mut count_nodes_in_layers = 0usize;
    for layer in &g.layers {
      count_nodes_in_layers += layer.len();
    }
    assert_eq!(count_nodes_in_layers, num_nodes);

    // async flag lengths align with adjacency
    for n in 0..num_nodes {
      assert_eq!(g.adjacency[n].len(), g.async_edge[n].len());
      for (&t, &is_async) in g.adjacency[n].iter().zip(g.async_edge[n].iter()) {
        assert!(t < num_nodes, "edge target out of range");
        let _ = is_async; // ensure we iterate both vectors
      }
    }

    // acyclicity
    assert_is_dag(&g);

    // edges go to deeper layers only
    let depth_map = layer_index_map(&g.layers, num_nodes);
    for u in 0..num_nodes {
      for &v in &g.adjacency[u] {
        assert!(
          depth_map[v] > depth_map[u],
          "edge not forward: {} -> {}",
          u,
          v
        );
      }
    }

    // every node not in the first layer has indegree >= 1
    let mut indeg = vec![0usize; num_nodes];
    for u in 0..num_nodes {
      for &v in &g.adjacency[u] {
        indeg[v] += 1;
      }
    }
    for &n in &g.layers[0] {
      // roots can be zero
      let _ = n;
    }
    for d in 1..g.layers.len() {
      for &n in &g.layers[d] {
        assert!(
          indeg[n] >= 1,
          "node at depth {} has no incoming edges: {}",
          d,
          n
        );
      }
    }
  }

  #[test]
  fn build_dag_async_ratio_zero_and_one() {
    let num_nodes = 30;
    let mut rng = StdRng::seed_from_u64(999);
    // ratio = 1.0 -> all edges async
    let g_all_async = build_dag(BuildDagParams {
      num_nodes,
      max_depth: 5,
      avg_out_degree: 2.5,
      rng: &mut rng,
      layer_weights: None,
      geo_p: None,
      subtrees: 3,
      cross_edge_prob: 0.4,
      cluster_weights: None,
      async_import_ratio: 1.0,
    });
    let mut total_edges = 0usize;
    let mut async_edges = 0usize;
    for n in 0..num_nodes {
      for i in 0..g_all_async.adjacency[n].len() {
        total_edges += 1;
        if g_all_async.async_edge[n][i] {
          async_edges += 1;
        }
      }
    }
    if total_edges > 0 {
      assert_eq!(total_edges, async_edges);
    }

    // ratio = 0.0 -> no edges async
    let mut rng2 = StdRng::seed_from_u64(999);
    let g_none_async = build_dag(BuildDagParams {
      num_nodes,
      max_depth: 5,
      avg_out_degree: 2.5,
      rng: &mut rng2,
      layer_weights: None,
      geo_p: None,
      subtrees: 3,
      cross_edge_prob: 0.4,
      cluster_weights: None,
      async_import_ratio: 0.0,
    });
    let mut any_async = false;
    for n in 0..num_nodes {
      for &flag in &g_none_async.async_edge[n] {
        if flag {
          any_async = true;
          break;
        }
      }
    }
    assert!(!any_async);
  }

  // These tests produce DOT and SVG artifacts under target/graph-tests for manual inspection.
  // They are ignored by default to avoid requiring graphviz in CI. Run with: cargo test -p atlaspack_benchmark -- --ignored
  #[test]
  #[ignore]
  fn generate_svg_depth_one_and_chain_examples() {
    let mut rng = StdRng::seed_from_u64(1337);
    let g_depth1 = build_dag(BuildDagParams {
      num_nodes: 20,
      max_depth: 1,
      avg_out_degree: 2.0,
      rng: &mut rng,
      layer_weights: None,
      geo_p: None,
      subtrees: 3,
      cross_edge_prob: 0.1,
      cluster_weights: None,
      async_import_ratio: 0.0,
    });
    write_svg("depth1_no_edges", &g_depth1);

    let g_chain = build_dag(BuildDagParams {
      num_nodes: 25,
      max_depth: 100,
      avg_out_degree: 3.0,
      rng: &mut rng,
      layer_weights: None,
      geo_p: None,
      subtrees: 2,
      cross_edge_prob: 0.5,
      cluster_weights: None,
      async_import_ratio: 0.0,
    });
    write_svg("chain_layers", &g_chain);
  }

  #[test]
  #[ignore]
  fn generate_svg_various_random_graphs() {
    let mut rng = StdRng::seed_from_u64(4242);
    // Balanced layers via geometric weights
    let g_geo = build_dag(BuildDagParams {
      num_nodes: 200,
      max_depth: 8,
      avg_out_degree: 2.3,
      rng: &mut rng,
      layer_weights: None,
      geo_p: Some(0.35),
      subtrees: 5,
      cross_edge_prob: 0.2,
      cluster_weights: None,
      async_import_ratio: 0.15,
    });
    write_svg("geo_weights", &g_geo);

    // Custom layer weights (shorter than depth, should be normalized and padded)
    let weights = vec![5.0, 3.0, 2.0];
    let g_weights = build_dag(BuildDagParams {
      num_nodes: 180,
      max_depth: 7,
      avg_out_degree: 2.8,
      rng: &mut rng,
      layer_weights: Some(&weights),
      geo_p: None,
      subtrees: 4,
      cross_edge_prob: 0.3,
      cluster_weights: None,
      async_import_ratio: 0.05,
    });
    write_svg("custom_layer_weights", &g_weights);

    // Many subtrees and high cross-subtree probability
    let g_cross = build_dag(BuildDagParams {
      num_nodes: 250,
      max_depth: 7,
      avg_out_degree: 3.1,
      rng: &mut rng,
      layer_weights: None,
      geo_p: None,
      subtrees: 8,
      cross_edge_prob: 0.9,
      cluster_weights: None,
      async_import_ratio: 0.25,
    });
    write_svg("many_subtrees_high_cross", &g_cross);
  }
}
