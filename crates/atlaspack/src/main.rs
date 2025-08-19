use atlaspack::{
  requests::{
    bundle_graph_request::{BundleGraphRequest, BundleGraphRequestOutput},
    package_request::{PackageRequest},
    AssetGraphRequest,
  },
  test_utils::{create_db, get_core_path},
  Atlaspack, AtlaspackInitOptions,
};
use atlaspack_core::types::{AtlaspackOptions, BuildMode, DefaultTargetOptions, FeatureFlagValue, FeatureFlags};
use atlaspack_monitoring::{MonitoringOptions, TracerMode};
use atlaspack_plugin_rpc::rust::RustWorkerFactory;
use clap::Parser;
use std::{path::PathBuf, sync::Arc};
use tracing::{error, info};

#[derive(Parser)]
struct Args {
  #[arg(short, long)]
  dev: bool,

  #[arg(short, long)]
  config: Option<PathBuf>,

  // atlaspack <ENTRIES>
  #[arg(trailing_var_arg = true)]
  entries: Vec<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
  initialize_tracing();

  info!("This is a testing binary only and requires a check-out of the atlaspack repository.");

  let args = Args::parse();
  run(args).await.unwrap_or_else(|e| {
    error!("Failed to run atlaspack: {}", e);
    std::process::exit(1);
  });
}

async fn run(args: Args) -> anyhow::Result<()> {
  let atlaspack = make_atlaspack(&args).await?;
  let atlaspack = Arc::new(atlaspack);

  info!("Building asset graph");
  atlaspack
    .run_request_async(AssetGraphRequest::default())
    .await?;

  info!("Building bundle graph");
  let result = atlaspack
    .run_request_async(BundleGraphRequest::default())
    .await?;
  let BundleGraphRequestOutput {
    bundle_graph,
    asset_graph,
    ..
  } = result
    .into_bundle_graph()
    .ok_or_else(|| anyhow::anyhow!("Invalid request result from bundle graph request."))?;

  let asset_graph = Arc::new(asset_graph);

  info!("Packaging bundles");
  let output_dir = PathBuf::from("dist");
  std::fs::create_dir_all(&output_dir)?;

  let request = PackageRequest::new(bundle_graph, asset_graph.clone());
  atlaspack.run_request_async(request).await?;

  Ok(())
}

async fn make_atlaspack(args: &Args) -> anyhow::Result<Atlaspack> {
  let atlaspack = Atlaspack::new(AtlaspackInitOptions {
    db: create_db().unwrap(),
    fs: Some(Arc::new(atlaspack_resolver::OsFileSystem)),
    options: AtlaspackOptions {
      entries: args.entries.clone(),
      mode: if args.dev {
        BuildMode::Development
      } else {
        BuildMode::Production
      },
      config: args
        .config
        .as_ref()
        .map(|c| c.to_str().unwrap().to_string()),
      default_target_options: DefaultTargetOptions {
        should_optimize: Some(false),
        should_scope_hoist: Some(false),
        ..Default::default()
      },
      core_path: get_core_path(),
      feature_flags: Arc::new(FeatureFlags(
        [("native_everything", FeatureFlagValue::Bool(true))]
          .iter()
          .map(|(k, v)| (k.to_string(), v.clone()))
          .collect(),
      )),
      ..Default::default()
    },
    package_manager: None,
    rpc: Arc::new(RustWorkerFactory::new().await?),
  })?;

  Ok(atlaspack)
}

fn initialize_tracing() {
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var(
      "RUST_LOG",
      "info,swc_ecma_compat_es2022=warn,swc_ecma_codegen=warn",
    );
  }
  let mut options = MonitoringOptions::from_env().unwrap();
  if options.tracing_options.is_empty() {
    options.tracing_options.push(TracerMode::Stdout);
  }
  atlaspack_monitoring::initialize_monitoring(options).unwrap();
}
