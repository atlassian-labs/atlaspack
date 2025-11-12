#[cfg(test)]
mod incremental_tests {
  use std::path::{Path, PathBuf};
  use std::sync::Arc;

  use atlaspack_core::asset_graph::AssetGraph;
  use atlaspack_core::types::{Asset, AtlaspackOptions, Code};
  use atlaspack_filesystem::FileSystem;
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use crate::requests::asset_request;
  use crate::requests::{AssetGraphRequest, RequestResult};
  use crate::test_utils::{RequestTrackerTestOptions, request_tracker};

  fn create_test_asset(id: &str, content: &str, file_path: &str) -> Arc<Asset> {
    Arc::new(Asset {
      id: id.to_string(),
      file_path: PathBuf::from(file_path),
      unique_key: Some(id.to_string()),
      code: Code::from(content.to_string()),
      ..Asset::default()
    })
  }

  fn setup_test_fs(temp_dir: &Path) -> InMemoryFileSystem {
    let fs = InMemoryFileSystem::default();
    fs.create_directory(temp_dir).unwrap();
    fs.set_current_working_directory(temp_dir);
    fs.write_file(&temp_dir.join("package.json"), String::from("{}"));
    fs
  }

  #[test]
  fn test_incremental_rebuild_with_content_only_changes() {
    // This test directly uses apply_changed_assets to test the incremental functionality
    // without going through the full request tracker which has complex transformer requirements

    let mut initial_graph = AssetGraph::new();

    // Create initial asset
    let initial_asset = create_test_asset(
      "entry.js",
      "console.log('Initial content');",
      "/test/entry.js",
    );

    let asset_id = initial_graph.add_asset(initial_asset.clone(), false);
    initial_graph.add_edge(&initial_graph.root_node(), &asset_id);

    // Verify initial state
    assert_eq!(initial_graph.get_assets().count(), 1);
    assert!(
      !initial_graph.safe_to_skip_bundling,
      "Initial graph should not be marked for skipping bundling"
    );

    // Create updated asset with content-only changes
    let updated_asset = create_test_asset(
      "entry.js",
      "console.log('Updated content with additional log');",
      "/test/entry.js",
    );

    // Apply the changed asset using the incremental functionality
    let updated_graph = initial_graph
      .apply_changed_assets(vec![updated_asset.clone()])
      .expect("apply_changed_assets should succeed");

    // Verify incremental optimization worked
    assert!(
      updated_graph.safe_to_skip_bundling,
      "Graph should be marked as safe to skip bundling after incremental update"
    );

    // Asset count should remain the same
    assert_eq!(updated_graph.get_assets().count(), 1);

    // The updated asset should have the new content
    let updated_asset_in_graph = updated_graph
      .get_assets()
      .find(|asset| asset.file_path.to_str().unwrap().contains("entry.js"))
      .expect("Should find entry.js asset");

    assert_eq!(
      updated_asset_in_graph.code,
      Code::from("console.log('Updated content with additional log');".to_string()),
      "Asset should have updated content"
    );

    // Verify that only the updated asset is tracked
    assert_eq!(
      updated_graph.updated_nodes().count(),
      1,
      "Should track exactly 1 updated node"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_incremental_rebuild_fails_with_dependency_changes() {
    let temp_dir = PathBuf::from("/atlaspack_dep_change_tests");

    let fs = setup_test_fs(&temp_dir);

    let initial_content = "console.log('content');";

    fs.write_file(&temp_dir.join("entry.js"), initial_content.to_string());
    fs.write_file(
      &temp_dir.join("helper.js"),
      "console.log('helper');".to_string(),
    );
    fs.write_file(
      &temp_dir.join("utils.js"),
      "console.log('utils');".to_string(),
    );

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      atlaspack_options: AtlaspackOptions {
        entries: vec![temp_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      fs: Arc::new(fs),
      project_root: temp_dir.clone(),
      search_path: temp_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    // Initial build
    let initial_request = AssetGraphRequest {
      prev_asset_graph: None,
      incrementally_bundled_assets: None,
    };

    let initial_result = request_tracker
      .run_request(initial_request)
      .await
      .expect("Initial build should succeed");

    let RequestResult::AssetGraph(initial_graph_result) = initial_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Simulate adding a new dependency
    let updated_content_with_new_dep = "console.log('content with new dependency');";

    let entry_asset = create_test_asset(
      "entry.js",
      updated_content_with_new_dep.trim(),
      temp_dir.join("entry.js").to_str().unwrap(),
    );

    // Create a mock dependency representing the new import
    use atlaspack_core::types::{DependencyBuilder, Environment, Priority, SpecifierType};

    let new_dependency = DependencyBuilder::default()
      .specifier("./utils.js".to_string())
      .env(Arc::new(Environment::default()))
      .specifier_type(SpecifierType::default())
      .source_path(temp_dir.join("entry.js"))
      .source_asset_id("entry.js".to_string())
      .priority(Priority::default())
      .build();

    let mock_asset_request_output = asset_request::AssetRequestOutput {
      asset: entry_asset.clone(),
      discovered_assets: vec![],
      dependencies: vec![new_dependency], // New dependency added
    };

    let incremental_request = AssetGraphRequest {
      prev_asset_graph: Some(initial_graph_result.graph.clone()),
      incrementally_bundled_assets: Some(vec![Arc::new(RequestResult::Asset(
        mock_asset_request_output,
      ))]),
    };

    let incremental_result = request_tracker
      .run_request(incremental_request)
      .await
      .expect("Build should succeed but do full rebuild");

    let RequestResult::AssetGraph(incremental_graph_result) = incremental_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Should have done a full rebuild, not incremental
    assert!(
      !incremental_graph_result.graph.safe_to_skip_bundling,
      "Graph should NOT be marked as safe to skip bundling due to dependency changes"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_incremental_rebuild_fails_with_discovered_asset_changes() {
    let temp_dir = PathBuf::from("/atlaspack_discovered_tests");

    let fs = setup_test_fs(&temp_dir);

    let initial_content = "console.log('initial');";

    fs.write_file(&temp_dir.join("entry.js"), initial_content.to_string());

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      atlaspack_options: AtlaspackOptions {
        entries: vec![temp_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      fs: Arc::new(fs),
      project_root: temp_dir.clone(),
      search_path: temp_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    // Initial build
    let initial_request = AssetGraphRequest {
      prev_asset_graph: None,
      incrementally_bundled_assets: None,
    };

    let initial_result = request_tracker
      .run_request(initial_request)
      .await
      .expect("Initial build should succeed");

    let RequestResult::AssetGraph(initial_graph_result) = initial_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Simulate discovered asset changes
    let entry_asset = create_test_asset(
      "entry.js",
      "console.log('updated');",
      temp_dir.join("entry.js").to_str().unwrap(),
    );

    // Create a mock discovered asset
    let discovered_asset = atlaspack_core::types::AssetWithDependencies {
      asset: Asset {
        id: "discovered.js".to_string(),
        file_path: temp_dir.join("discovered.js"),
        unique_key: Some("discovered.js".to_string()),
        code: Code::from("// discovered asset".to_string()),
        ..Asset::default()
      },
      dependencies: vec![],
    };

    let mock_asset_request_output = asset_request::AssetRequestOutput {
      asset: entry_asset.clone(),
      discovered_assets: vec![discovered_asset], // New discovered asset
      dependencies: vec![],
    };

    let incremental_request = AssetGraphRequest {
      prev_asset_graph: Some(initial_graph_result.graph.clone()),
      incrementally_bundled_assets: Some(vec![Arc::new(RequestResult::Asset(
        mock_asset_request_output,
      ))]),
    };

    let incremental_result = request_tracker
      .run_request(incremental_request)
      .await
      .expect("Build should succeed but do full rebuild");

    let RequestResult::AssetGraph(incremental_graph_result) = incremental_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Should have done a full rebuild, not incremental
    assert!(
      !incremental_graph_result.graph.safe_to_skip_bundling,
      "Graph should NOT be marked as safe to skip bundling due to discovered asset changes"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_incremental_rebuild_fails_with_virtual_assets() {
    let temp_dir = PathBuf::from("/atlaspack_virtual_tests");

    let fs = setup_test_fs(&temp_dir);

    let initial_content = "console.log('initial');";
    fs.write_file(&temp_dir.join("entry.js"), initial_content.to_string());

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      atlaspack_options: AtlaspackOptions {
        entries: vec![temp_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      fs: Arc::new(fs),
      project_root: temp_dir.clone(),
      search_path: temp_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    // Initial build
    let initial_request = AssetGraphRequest {
      prev_asset_graph: None,
      incrementally_bundled_assets: None,
    };

    let initial_result = request_tracker
      .run_request(initial_request)
      .await
      .expect("Initial build should succeed");

    let RequestResult::AssetGraph(initial_graph_result) = initial_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Create a virtual asset
    let virtual_asset = Arc::new(Asset {
      id: "virtual:entry".to_string(),
      file_path: PathBuf::from("virtual:entry"),
      unique_key: Some("virtual:entry".to_string()),
      code: Code::from("console.log('virtual');".to_string()),
      is_virtual: true, // This is the key difference
      ..Asset::default()
    });

    let mock_asset_request_output = asset_request::AssetRequestOutput {
      asset: virtual_asset.clone(),
      discovered_assets: vec![],
      dependencies: vec![],
    };

    let incremental_request = AssetGraphRequest {
      prev_asset_graph: Some(initial_graph_result.graph.clone()),
      incrementally_bundled_assets: Some(vec![Arc::new(RequestResult::Asset(
        mock_asset_request_output,
      ))]),
    };

    let incremental_result = request_tracker
      .run_request(incremental_request)
      .await
      .expect("Build should succeed but do full rebuild");

    let RequestResult::AssetGraph(incremental_graph_result) = incremental_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Should have done a full rebuild, not incremental
    assert!(
      !incremental_graph_result.graph.safe_to_skip_bundling,
      "Graph should NOT be marked as safe to skip bundling for virtual assets"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_incremental_rebuild_with_no_prev_asset_graph() {
    let temp_dir = PathBuf::from("/atlaspack_no_prev_tests");

    let fs = setup_test_fs(&temp_dir);
    fs.write_file(
      &temp_dir.join("entry.js"),
      "console.log('test');".to_string(),
    );

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      atlaspack_options: AtlaspackOptions {
        entries: vec![temp_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      fs: Arc::new(fs),
      project_root: temp_dir.clone(),
      search_path: temp_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    let entry_asset = create_test_asset(
      "entry.js",
      "console.log('test');",
      temp_dir.join("entry.js").to_str().unwrap(),
    );

    let mock_asset_request_output = asset_request::AssetRequestOutput {
      asset: entry_asset.clone(),
      discovered_assets: vec![],
      dependencies: vec![],
    };

    // Test with no previous asset graph
    let incremental_request = AssetGraphRequest {
      prev_asset_graph: None, // No previous graph
      incrementally_bundled_assets: Some(vec![Arc::new(RequestResult::Asset(
        mock_asset_request_output,
      ))]),
    };

    let result = request_tracker
      .run_request(incremental_request)
      .await
      .expect("Build should succeed but do full rebuild");

    let RequestResult::AssetGraph(graph_result) = result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Should have done a full rebuild
    assert!(
      !graph_result.graph.safe_to_skip_bundling,
      "Graph should NOT be marked as safe to skip bundling without previous graph"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_incremental_rebuild_with_no_incrementally_bundled_assets() {
    let temp_dir = PathBuf::from("/atlaspack_no_incremental_tests");

    let fs = setup_test_fs(&temp_dir);
    fs.write_file(
      &temp_dir.join("entry.js"),
      "console.log('test');".to_string(),
    );

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      atlaspack_options: AtlaspackOptions {
        entries: vec![temp_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      fs: Arc::new(fs),
      project_root: temp_dir.clone(),
      search_path: temp_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    // First build to get a previous asset graph
    let initial_request = AssetGraphRequest {
      prev_asset_graph: None,
      incrementally_bundled_assets: None,
    };

    let initial_result = request_tracker
      .run_request(initial_request)
      .await
      .expect("Initial build should succeed");

    let RequestResult::AssetGraph(initial_graph_result) = initial_result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Test with no incrementally bundled assets
    let incremental_request = AssetGraphRequest {
      prev_asset_graph: Some(initial_graph_result.graph.clone()),
      incrementally_bundled_assets: None, // No incremental assets
    };

    let result = request_tracker
      .run_request(incremental_request)
      .await
      .expect("Build should succeed but do full rebuild");

    let RequestResult::AssetGraph(graph_result) = result.as_ref() else {
      panic!("Expected AssetGraphRequestOutput");
    };

    // Should have done a full rebuild
    assert!(
      !graph_result.graph.safe_to_skip_bundling,
      "Graph should NOT be marked as safe to skip bundling without incremental assets"
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_apply_changed_assets_preserves_graph_structure() {
    let mut initial_graph = AssetGraph::new();

    // Create initial assets
    let asset1 = create_test_asset("asset1", "initial content 1", "/test/asset1.js");
    let asset2 = create_test_asset("asset2", "initial content 2", "/test/asset2.js");

    let asset1_id = initial_graph.add_asset(asset1.clone(), false);
    let asset2_id = initial_graph.add_asset(asset2.clone(), false);

    // Add some graph structure
    initial_graph.add_edge(&initial_graph.root_node(), &asset1_id);
    initial_graph.add_edge(&asset1_id, &asset2_id);

    let initial_asset_count = initial_graph.get_assets().count();
    let initial_edges = initial_graph.edges();

    // Apply changed assets with updated content
    let updated_asset1 = create_test_asset("asset1", "updated content 1", "/test/asset1.js");
    let updated_asset2 = create_test_asset("asset2", "updated content 2", "/test/asset2.js");

    let updated_graph = initial_graph
      .apply_changed_assets(vec![updated_asset1.clone(), updated_asset2.clone()])
      .expect("apply_changed_assets should succeed");

    // Verify structure is preserved
    assert_eq!(
      updated_graph.get_assets().count(),
      initial_asset_count,
      "Asset count should remain the same"
    );

    assert_eq!(
      updated_graph.edges(),
      initial_edges,
      "Graph edges should be preserved"
    );

    assert!(
      updated_graph.safe_to_skip_bundling,
      "Graph should be marked as safe to skip bundling"
    );

    // Verify content is updated
    let updated_asset1_in_graph = updated_graph
      .get_assets()
      .find(|asset| asset.id == "asset1")
      .expect("Should find updated asset1");

    assert_eq!(
      updated_asset1_in_graph.code,
      Code::from("updated content 1".to_string()),
      "Asset content should be updated"
    );

    // Verify node delta tracking
    assert_eq!(
      updated_graph.updated_nodes().count(),
      2,
      "Should track 2 updated nodes"
    );
  }
}
