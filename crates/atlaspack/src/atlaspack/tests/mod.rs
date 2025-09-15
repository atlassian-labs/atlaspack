use std::collections::{HashMap, HashSet};

use atlaspack_benchmark::GenerateMonorepoParams;
use atlaspack_core::{
  bundle_graph::BundleGraphNode,
  types::{Asset, AssetId, BuildMode, Code},
};
use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
use atlaspack_plugin_rpc::{MockRpcFactory, MockRpcWorker};
use tracing::info;

use crate::{
  requests::{
    bundle_graph_request::{self, BundleGraphRequestOutput},
    package_request::{PackageRequest, PackageRequestOutput},
  },
  test_utils,
};

use super::*;

#[test]
fn build_asset_graph_commits_assets_to_lmdb() -> Result<(), anyhow::Error> {
  // TODO: Create overlay fs for integration test
  let db = test_utils::create_db()?;
  let fs = InMemoryFileSystem::default();

  fs.write_file(
    &PathBuf::from("/.parcelrc"),
    r#"
      {
        "bundler": "",
        "namers": [""],
        "resolvers": [""],
      }
    "#
    .to_string(),
  );

  let atlaspack = Atlaspack::new(AtlaspackInitOptions {
    db: db.clone(),
    fs: Some(Arc::new(fs)),
    options: AtlaspackOptions::default(),
    package_manager: None,
    rpc: rpc(),
  })?;

  let assets_names = ["foo", "bar", "baz"];
  let assets = assets_names
    .iter()
    .enumerate()
    .map(|(idx, asset)| {
      AssetGraphNode::Asset(AssetNode {
        asset: Asset {
          id: AssetId::from(idx as u64),
          code: Code::from(asset.to_string()),
          ..Asset::default()
        },
        requested_symbols: HashSet::new(),
      })
    })
    .collect::<Vec<AssetGraphNode>>();

  atlaspack.commit_assets(assets.iter().collect())?;

  let txn = db.database().read_txn()?;
  for (idx, asset) in assets_names.iter().enumerate() {
    let entry = db
      .database()
      .get(&txn, &AssetId::from(idx as u64).to_string())?;
    assert_eq!(entry, Some(asset.to_string().into()));
  }

  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_create_bundle_from_non_library_app() -> anyhow::Result<()> {
  let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
    .with_max_level(tracing::Level::DEBUG)
    .try_init();

  let project_dir =
    test_utils::setup_test_directory("test-create-asset-graph-from-non-library-app")?;

  std::fs::create_dir_all(project_dir.join("src"))?;
  std::fs::write(
    project_dir.join(".parcelrc"),
    r#"
{
  "extends": "@atlaspack/config-default"
}
      "#,
  )?;

  std::fs::write(
    project_dir.join("tsconfig.json"),
    r#"
{
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
}
      "#,
  )
  .unwrap();
  std::fs::write(
    project_dir.join("package.json"),
    r#"
{
  "name": "simple-app"
}
      "#,
  )
  .unwrap();
  std::fs::write(
    project_dir.join("src/index.ts"),
    r#"
import { bar } from "./bar";

output(bar + "foo");
      "#,
  )
  .unwrap();

  std::fs::write(
    project_dir.join("src/bar.ts"),
    r#"
export const bar = "bar";
      "#,
  )
  .unwrap();

  let atlaspack = test_utils::make_test_atlaspack(&[project_dir.join("src/index.ts")])
    .await
    .unwrap();

  let _asset_graph = atlaspack.build_asset_graph_async().await?;
  let _bundle_graph = atlaspack
    .run_request_async(bundle_graph_request::BundleGraphRequest {})
    .await?;
  let BundleGraphRequestOutput {
    bundle_graph,
    asset_graph,
  } = atlaspack
    .run_request_async(bundle_graph_request::BundleGraphRequest {})
    .await
    .unwrap()
    .into_bundle_graph()
    .unwrap();
  assert_eq!(bundle_graph.num_bundles(), 1);
  let BundleGraphNode::Bundle(_bundle) = bundle_graph
    .graph()
    .node_weights()
    .find(|weight| matches!(weight, BundleGraphNode::Bundle(_)))
    .unwrap()
  else {
    panic!("Expected a bundle");
  };

  let output = Vec::new();
  let asset_graph = Arc::new(asset_graph);
  // package_request::package_bundle(
  //   package_request::PackageBundleParams {
  //     bundle: &bundle,
  //     bundle_node_index: bundle_graph.graph().node_indices().next().unwrap(),
  //   },
  //   &InMemoryAssetDataProvider::new(asset_graph.clone()),
  //   &mut output,
  //   &bundle_graph,
  //   None,
  // )?;

  let code = String::from_utf8(output).unwrap();
  println!("{}", code);

  tracing::debug!("Asset graph: {:#?}", asset_graph);

  let assets = asset_graph
    .nodes()
    .filter_map(|node| {
      if let AssetGraphNode::Asset(node) = node {
        Some(node)
      } else {
        None
      }
    })
    .collect::<Vec<_>>();

  let assets = assets
    .iter()
    .map(|asset| (asset.asset.file_path.clone(), asset.asset.clone()))
    .collect::<HashMap<_, _>>();

  tracing::debug!(test_path = ?test_utils::get_repo_path().join("packages/transformers/js/src/esmodule-helpers.js"), "Assets");
  tracing::debug!(assets = ?assets.keys().collect::<Vec<_>>(), "Assets");

  assert_eq!(assets.len(), 2);
  assert!(assets.contains_key(&project_dir.join("src/bar.ts")));
  assert!(assets.contains_key(&project_dir.join("src/index.ts")));
  // assert!(assets.contains_key(
  //   &test_utils::get_repo_path().join("packages/transformers/js/src/esmodule-helpers.js")
  // ));

  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_bundle_async_import() -> anyhow::Result<()> {
  let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
    .with_max_level(tracing::Level::DEBUG)
    .try_init();

  let project_dir = test_utils::setup_test_directory("test-bundle-async-import")?;
  std::fs::write(
    project_dir.join("index.js"),
    r#"
      const {e} = await import("./async.js");
      console.log(e);
      "#,
  )?;
  std::fs::write(
    project_dir.join("async.js"),
    r#"
      console.log("Hello, world!");
      export const e = "e";
      "#,
  )?;

  let atlaspack = test_utils::make_test_atlaspack(&[project_dir.join("index.js")]).await?;

  let BundleGraphRequestOutput {
    bundle_graph,
    asset_graph,
  } = atlaspack
    .run_request_async(bundle_graph_request::BundleGraphRequest {})
    .await?
    .into_bundle_graph()
    .unwrap();
  let asset_graph = Arc::new(asset_graph);

  assert_eq!(bundle_graph.num_bundles(), 2);

  let PackageRequestOutput { bundle_paths } = atlaspack
    .run_request_async(PackageRequest::new(bundle_graph, asset_graph.clone()))
    .await?
    .into_package()
    .unwrap();

  for (bundle_id, bundle_path) in bundle_paths {
    println!("Bundle ID: {}", bundle_id);
    let code = std::fs::read_to_string(bundle_path)?;
    println!("{}", code);
  }

  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_bundle_html_file() -> anyhow::Result<()> {
  let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
    .with_max_level(tracing::Level::DEBUG)
    .try_init();

  let project_dir = test_utils::setup_test_directory("test-bundle-html-file")?;
  std::fs::write(
    project_dir.join("index.html"),
    r#"
      <html>
        <body>
          <script src="index.js"></script>
        </body>
      </html>
      "#,
  )?;
  std::fs::write(
    project_dir.join("index.js"),
    r#"
      console.log("Hello, world!");
      "#,
  )?;

  let atlaspack = test_utils::make_test_atlaspack(&[project_dir.join("index.html")]).await?;

  let BundleGraphRequestOutput {
    bundle_graph,
    asset_graph,
  } = atlaspack
    .run_request_async(bundle_graph_request::BundleGraphRequest {})
    .await?
    .into_bundle_graph()
    .unwrap();
  let asset_graph = Arc::new(asset_graph);

  assert_eq!(bundle_graph.num_bundles(), 2);

  let PackageRequestOutput { bundle_paths } = atlaspack
    .run_request_async(PackageRequest::new(bundle_graph, asset_graph.clone()))
    .await?
    .into_package()
    .unwrap();

  for (bundle_id, bundle_path) in bundle_paths {
    println!("Bundle ID: {}", bundle_id);
    let code = std::fs::read_to_string(bundle_path)?;
    println!("{}", code);
  }

  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_create_asset_graph_from_simple_app() -> anyhow::Result<()> {
  let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
    .with_max_level(tracing::Level::DEBUG)
    .try_init();

  let project_dir = test_utils::setup_test_directory("test-create-asset-graph-from-simple-app")?;

  std::fs::create_dir_all(project_dir.join("src"))?;
  std::fs::write(
    project_dir.join(".parcelrc"),
    r#"
{
  "extends": "@atlaspack/config-default"
}
      "#,
  )?;

  std::fs::write(
    project_dir.join("tsconfig.json"),
    r#"
{
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
}
      "#,
  )
  .unwrap();
  std::fs::write(
    project_dir.join("package.json"),
    r#"
{
  "name": "simple-app"
}
      "#,
  )
  .unwrap();
  std::fs::write(
    project_dir.join("src/index.ts"),
    r#"
import { bar } from "./bar";

document.body.innerHTML = bar + "foo";
      "#,
  )
  .unwrap();

  std::fs::write(
    project_dir.join("src/bar.ts"),
    r#"
export const bar = "bar";
      "#,
  )
  .unwrap();

  info!(?project_dir, "Creating atlaspack");
  let atlaspack =
    test_utils::make_test_atlaspack_with(&[project_dir.join("src/index.ts")], |init_options| {
      init_options.options.mode = BuildMode::Development;
      init_options.options.default_target_options.should_optimize = Some(false);
      init_options
        .options
        .default_target_options
        .should_scope_hoist = Some(false);
      init_options.options.default_target_options.is_library = Some(false);
      init_options.options.targets = None;
    })
    .await
    .unwrap();

  info!("Building asset graph");
  let _asset_graph = atlaspack.build_asset_graph_async().await?;
  let BundleGraphRequestOutput {
    bundle_graph,
    asset_graph,
  } = atlaspack
    .run_request_async(bundle_graph_request::BundleGraphRequest {})
    .await
    .unwrap()
    .into_bundle_graph()
    .unwrap();
  assert_eq!(bundle_graph.num_bundles(), 1);
  let asset_graph = Arc::new(asset_graph);

  let PackageRequestOutput { bundle_paths } = atlaspack
    .run_request_async(PackageRequest::new(bundle_graph, asset_graph.clone()))
    .await?
    .into_package()
    .unwrap();

  for (_, bundle_path) in bundle_paths {
    let code = std::fs::read_to_string(bundle_path)?;
    println!("{}", code);
  }

  // tracing::debug!("Asset graph: {:#?}", asset_graph);

  let assets = asset_graph
    .nodes()
    .filter_map(|node| {
      if let AssetGraphNode::Asset(node) = node {
        Some(node)
      } else {
        None
      }
    })
    .collect::<Vec<_>>();

  let assets = assets
    .iter()
    .map(|asset| (asset.asset.file_path.clone(), asset.asset.clone()))
    .collect::<HashMap<_, _>>();

  tracing::debug!(test_path = ?test_utils::get_repo_path().join("packages/transformers/js/src/esmodule-helpers.js"), "Assets");
  tracing::debug!(assets = ?assets.keys().collect::<Vec<_>>(), "Assets");

  assert_eq!(assets.len(), 2);
  assert!(assets.contains_key(&project_dir.join("src/bar.ts")));
  assert!(assets.contains_key(&project_dir.join("src/index.ts")));

  Ok(())
}

#[tokio::test]
#[ignore]
async fn test_build_asset_graph_from_large_synthetic_project() {
  let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
    .with_max_level(tracing::Level::INFO)
    .try_init();

  let project_dir =
    test_utils::setup_test_directory("test-build-asset-graph-from-large-synthetic-project")
      .unwrap();
  atlaspack_benchmark::generate_monorepo(GenerateMonorepoParams {
    num_files: 3,
    avg_lines_per_file: 0,
    depth: 50_000,
    subtrees: 10_000,
    target_dir: &project_dir,
    async_import_ratio: 0.0,
    ..Default::default()
  })
  .unwrap();

  let atlaspack = test_utils::make_test_atlaspack(&[project_dir.join("app-root/src/index.ts")])
    .await
    .unwrap();

  let BundleGraphRequestOutput {
    bundle_graph,
    asset_graph,
  } = atlaspack
    .run_request_async(bundle_graph_request::BundleGraphRequest {})
    .await
    .unwrap()
    .into_bundle_graph()
    .unwrap();

  assert_eq!(bundle_graph.num_bundles(), 1);
  let BundleGraphNode::Bundle(bundle) = bundle_graph
    .graph()
    .node_weights()
    .find(|weight| matches!(weight, BundleGraphNode::Bundle(_)))
    .unwrap()
  else {
    panic!("Expected a bundle");
  };

  let mut output = Vec::new();

  // package_request::package_bundle(
  //   &bundle,
  //   bundle_graph.graph().node_indices().next().unwrap(),
  //   &InMemoryAssetDataProvider::new(Arc::new(asset_graph)),
  //   &mut output,
  //   &bundle_graph,
  //   None,
  // )
  // .unwrap();

  let code = String::from_utf8(output).unwrap();
  println!("{}", code);

  let _ = std::fs::remove_dir_all(&project_dir);
}

fn rpc() -> RpcFactoryRef {
  let mut rpc_factory = MockRpcFactory::new();

  rpc_factory
    .expect_start()
    .returning(|| Ok(Arc::new(MockRpcWorker::new())));

  Arc::new(rpc_factory)
}
