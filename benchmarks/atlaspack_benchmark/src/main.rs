use std::path::PathBuf;

use anyhow::Context;
use atlaspack_benchmark::{generate_monorepo, GenerateMonorepoParams};
use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "atlaspack-benchmark")]
#[command(about = "Atlaspack benchmarks and utilities", long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
  /// Generate a JavaScript monorepo for benchmarking
  GenerateProject {
    /// Target directory for the monorepo
    target: PathBuf,
    /// Total number of source files (nodes) to generate
    #[arg(long = "files", short = 'n')]
    files: usize,
    /// Average lines of code per file
    #[arg(long = "avg-lines-per-file", short = 'l')]
    avg_lines_per_file: usize,
    /// Desired overall graph depth (number of layers)
    #[arg(long = "depth", short = 'd', default_value_t = 5)]
    depth: usize,
    /// Approximate average out-degree per node (number of imports per file), can be fractional like 1.2
    #[arg(long = "avg-out-degree", short = 'o', default_value_t = 2.0)]
    avg_out_degree: f64,
    /// Optional per-layer weights (CSV) controlling probability a node is placed at each depth layer (normalized automatically)
    #[arg(long = "layer-weights", value_delimiter = ',')]
    layer_weights: Option<Vec<f64>>,
    /// Optional geometric distribution parameter p for layer depth when no weights provided (0<p<=1)
    #[arg(long = "geo-p")]
    geo_p: Option<f64>,
    /// Optional RNG seed for reproducible graphs
    #[arg(long = "seed")]
    seed: Option<u64>,
    /// Optional output path for the PNG when using --dot-only. Defaults to <target>/graph.png
    #[arg(long = "dot-output")]
    dot_output: Option<PathBuf>,
    /// Number of subtrees/clusters to partition nodes into
    #[arg(long = "subtrees", default_value_t = 4)]
    subtrees: usize,
    /// Probability an edge crosses into a different subtree (0.0 - 1.0)
    #[arg(long = "cross-edge-prob", default_value_t = 0.05)]
    cross_edge_prob: f64,
    /// Optional per-subtree weights (CSV) controlling node assignment to subtrees
    #[arg(long = "cluster-weights", value_delimiter = ',')]
    cluster_weights: Option<Vec<f64>>,
    /// Ratio of edges to turn into async dynamic imports (0.0 - 1.0)
    #[arg(long = "async-import-ratio", default_value_t = 0.001)]
    async_import_ratio: f64,
  },
}

fn main() -> anyhow::Result<()> {
  let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
  tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_target(false)
    .compact()
    .init();

  let cli = Cli::parse();
  info!(command = ?cli.command, "starting command");
  match cli.command {
    Commands::GenerateProject {
      target,
      files,
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
    } => generate_monorepo(GenerateMonorepoParams {
      target_dir: &target,
      num_files: files,
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
    })
    .with_context(|| format!("Failed to generate monorepo at {}", target.display()))?,
  }
  Ok(())
}
