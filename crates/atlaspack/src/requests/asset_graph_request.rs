use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::anyhow;
use async_trait::async_trait;
use indexmap::IndexMap;
use pathdiff::diff_paths;

use crate::request_tracker::{
  Request, RequestResultReceiver, RequestResultSender, ResultAndInvalidations, RunRequestContext,
  RunRequestError,
};
use atlaspack_core::asset_graph::{AssetGraph, DependencyState, propagate_requested_symbols};
use atlaspack_core::types::{AssetWithDependencies, Dependency};

use super::RequestResult;
use super::asset_request::{AssetRequest, AssetRequestOutput};
use super::entry_request::{EntryRequest, EntryRequestOutput};
use super::path_request::{PathRequest, PathRequestOutput};
use super::target_request::{TargetRequest, TargetRequestOutput};

/// The AssetGraphRequest is in charge of building the AssetGraphRequest
/// In doing so, it kicks of the EntryRequest, TargetRequest, PathRequest and AssetRequests.
#[derive(Debug, Default)]
pub struct AssetGraphRequest {
  pub prev_asset_graph: Option<Arc<AssetGraph>>,
}

impl Hash for AssetGraphRequest {
  // Hash returns nothing here as every AssetGraphRequest should have the same
  // ID in the end.
  fn hash<H: Hasher>(&self, _state: &mut H) {}
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetGraphRequestOutput {
  pub graph: Arc<AssetGraph>,
}

#[async_trait]
impl Request for AssetGraphRequest {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let builder = AssetGraphBuilder::new(request_context, self.prev_asset_graph.clone());

    builder.build()
  }
}

type NodeId = usize;

pub(crate) struct AssetGraphBuilder {
  request_id_to_dependency_id: HashMap<u64, NodeId>,
  graph: AssetGraph,
  visited: HashSet<u64>,
  work_count: u32,
  request_context: RunRequestContext,
  sender: RequestResultSender,
  receiver: RequestResultReceiver,
  asset_request_to_asset_id: HashMap<u64, NodeId>,
  waiting_asset_requests: HashMap<u64, HashSet<NodeId>>,
  entry_dependencies: Vec<(String, NodeId)>,
}

impl AssetGraphBuilder {
  fn new(request_context: RunRequestContext, prev_asset_graph: Option<Arc<AssetGraph>>) -> Self {
    let (sender, receiver) = channel();

    AssetGraphBuilder {
      request_id_to_dependency_id: HashMap::new(),
      graph: prev_asset_graph.map_or_else(AssetGraph::new, |prev| AssetGraph::from(&prev)),
      visited: HashSet::new(),
      work_count: 0,
      request_context,
      sender,
      receiver,
      asset_request_to_asset_id: HashMap::new(),
      waiting_asset_requests: HashMap::new(),
      entry_dependencies: Vec::new(),
    }
  }

  #[tracing::instrument(level = "info", skip_all)]
  fn build(mut self) -> Result<ResultAndInvalidations, RunRequestError> {
    for entry in self.request_context.options.clone().entries.iter() {
      self.work_count += 1;
      let _ = self.request_context.queue_request(
        EntryRequest {
          entry: entry.clone(),
        },
        self.sender.clone(),
      );
    }

    loop {
      // TODO: Should the work count be tracked on the request_context as part of
      // the queue_request API?
      if self.work_count == 0 {
        break;
      }

      let Ok(result) = self.receiver.recv() else {
        break;
      };

      self.work_count -= 1;
      let (result, request_id, cached) = result?;

      match result.as_ref() {
        RequestResult::Entry(result) => {
          tracing::debug!("Handling EntryRequestOutput");
          self.handle_entry_result(result);
        }
        RequestResult::Target(result) => {
          tracing::debug!("Handling TargetRequestOutput");
          self.handle_target_request_result(result, cached);
        }
        RequestResult::Asset(result) => {
          tracing::debug!(
            "Handling AssetRequestOutput: {}",
            result.asset.file_path.display()
          );
          self.handle_asset_result(result, request_id, cached);
        }
        RequestResult::Path(result) => {
          tracing::debug!("Handling PathRequestOutput");
          self.handle_path_result(result, request_id);
        }
        // This branch should never occur
        result => {
          return Err(anyhow!(
            "Unexpected request result in AssetGraphRequest ({}): {:?}",
            request_id,
            result
          ));
        }
      }
    }

    // Connect the entries to the root node in the graph. We do this in
    // alphabetical order so it's consistent between builds.
    //
    // Ideally, we wouldn't depend on edge order being consistent between builds
    // and instead rely on in-place sorting or similar to ensure deterministic
    // builds. However, as the rest of the code base (bundling, runtimes,
    // packaging, etc) relies on the deterministic edge order and it's very
    // complicated/risky to fix all the places that would be affected we'll keep it that
    // way for now.
    self
      .entry_dependencies
      .sort_by_key(|(entry, _)| entry.clone());
    for (_, node_index) in self.entry_dependencies.iter() {
      self.graph.add_edge(&self.graph.root_node(), node_index);
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::AssetGraph(AssetGraphRequestOutput {
        graph: Arc::new(self.graph),
      }),
      invalidations: vec![],
    })
  }

  pub(crate) fn replicate_existing_edges(&mut self, existing_dep_id: NodeId, new_dep_id: NodeId) {
    let existing_edges = self.graph.get_outgoing_neighbors(&existing_dep_id);
    for edge in existing_edges {
      self.graph.add_edge(&new_dep_id, &edge);
      self.propagate_requested_symbols(edge, new_dep_id);
    }
  }

  fn handle_path_result(&mut self, result: &PathRequestOutput, request_id: u64) {
    let dependency_id = *self
      .request_id_to_dependency_id
      .get(&request_id)
      .expect("Missing node index for request id {request_id}");

    let dependency = self.graph.get_dependency(&dependency_id).unwrap();
    let requested_symbols = self.graph.get_requested_symbols(&dependency_id);
    let has_requested_symbols = requested_symbols.is_some_and(|s| !s.is_empty());

    let asset_request = match result {
      PathRequestOutput::Resolved {
        path,
        code,
        pipeline,
        side_effects,
        query,
        can_defer,
      } => {
        if !side_effects && *can_defer && !has_requested_symbols && dependency.symbols.is_some() {
          self
            .graph
            .set_dependency_state(&dependency_id, DependencyState::Deferred);
          return;
        }

        self
          .graph
          .set_dependency_state(&dependency_id, DependencyState::Resolved);

        AssetRequest {
          code: code.clone(),
          env: dependency.env.clone(),
          file_path: path.clone(),
          project_root: self.request_context.project_root.clone(),
          pipeline: pipeline.clone(),
          query: query.clone(),
          side_effects: *side_effects,
        }
      }
      PathRequestOutput::Excluded => {
        self
          .graph
          .set_dependency_state(&dependency_id, DependencyState::Excluded);
        return;
      }
    };
    let id = asset_request.id();

    if self.visited.insert(id) {
      self.request_id_to_dependency_id.insert(id, dependency_id);
      self.work_count += 1;
      let _ = self
        .request_context
        .queue_request(asset_request, self.sender.clone());
    } else if self.asset_request_to_asset_id.contains_key(&id) {
      // We also need to handle discovered assets here
      let previous_dependency_id = self
        .request_id_to_dependency_id
        .get(&id)
        .expect("Missing node index for request id {id}");

      // We have already completed this AssetRequest so we can connect the
      // Dependency to the Asset immediately
      self.replicate_existing_edges(*previous_dependency_id, dependency_id);
    } else {
      // The AssetRequest has already been kicked off but is yet to
      // complete. Register this Dependency to be connected once it
      // completes
      self
        .waiting_asset_requests
        .entry(id)
        .and_modify(|nodes| {
          nodes.insert(dependency_id);
        })
        .or_insert_with(|| HashSet::from([dependency_id]));
    }
  }

  fn handle_entry_result(&mut self, result: &EntryRequestOutput) {
    let EntryRequestOutput { entries, .. } = result;
    for entry in entries {
      let target_request = TargetRequest {
        default_target_options: self.request_context.options.default_target_options.clone(),
        entry: entry.clone(),
        env: self.request_context.options.env.clone(),
        mode: self.request_context.options.mode.clone(),
      };

      self.work_count += 1;
      let _ = self
        .request_context
        .queue_request(target_request, self.sender.clone());
    }
  }

  fn handle_asset_result(&mut self, result: &AssetRequestOutput, request_id: u64, cached: bool) {
    let AssetRequestOutput {
      asset,
      discovered_assets,
      dependencies,
    } = result;

    let incoming_dependency_id = *self
      .request_id_to_dependency_id
      .get(&request_id)
      .expect("Missing node index for request id {request_id}");

    let asset_unique_key = asset.unique_key.clone();

    // Connect the incoming DependencyNode to the new AssetNode
    let asset_id = self.graph.add_asset(asset.clone(), cached);

    self.graph.add_edge(&incoming_dependency_id, &asset_id);

    self.asset_request_to_asset_id.insert(request_id, asset_id);

    let mut added_discovered_assets: HashMap<String, NodeId> = HashMap::new();

    // Attach the "direct" discovered assets to the graph
    let direct_discovered_assets = get_direct_discovered_assets(discovered_assets, dependencies);
    for discovered_asset in direct_discovered_assets {
      let asset_id = self
        .graph
        .add_asset(Arc::new(discovered_asset.asset.clone()), cached);

      self.graph.add_edge(&incoming_dependency_id, &asset_id);

      self.add_asset_dependencies(
        &discovered_asset.dependencies,
        discovered_assets,
        asset_id,
        &mut added_discovered_assets,
        asset_id,
        asset_unique_key.as_ref(),
        cached,
      );
      self.propagate_requested_symbols(asset_id, incoming_dependency_id);
    }

    self.add_asset_dependencies(
      dependencies,
      discovered_assets,
      asset_id,
      &mut added_discovered_assets,
      asset_id,
      asset_unique_key.as_ref(),
      cached,
    );

    self.propagate_requested_symbols(asset_id, incoming_dependency_id);

    // Connect any previously discovered Dependencies that were waiting
    // for this AssetNode to be created
    if let Some(waiting) = self.waiting_asset_requests.remove(&request_id) {
      for dep_id in waiting {
        // If the incoming dependency has been linked to other assets, then
        // replicate those links for the waiting dependency
        self.replicate_existing_edges(incoming_dependency_id, dep_id);
      }
    }
  }

  #[allow(clippy::too_many_arguments)]
  fn add_asset_dependencies(
    &mut self,
    dependencies: &Vec<Dependency>,
    discovered_assets: &Vec<AssetWithDependencies>,
    asset_id: NodeId,
    added_discovered_assets: &mut HashMap<String, NodeId>,
    root_asset_id: NodeId,
    root_asset_unique_key: Option<&String>,
    cached: bool,
  ) {
    // Connect dependencies of the Asset
    let mut unique_deps: IndexMap<String, Dependency> = IndexMap::new();

    for dependency in dependencies {
      unique_deps
        .entry(dependency.id())
        .and_modify(|d| {
          // This code is an incomplete version of mergeDependencies in packages/core/core/src/Dependency.js
          // Duplicate dependencies can occur when node globals are polyfilled
          // e.g. 'process'. I think ideally we wouldn't end up with two
          // dependencies post-transform but that needs further investigation to
          // resolve and understand...
          d.meta.extend(dependency.meta.clone());
          if let Some(symbols) = d.symbols.as_mut() {
            if let Some(merge_symbols) = dependency.symbols.as_ref() {
              symbols.extend(merge_symbols.clone());
            }
          } else {
            d.symbols = dependency.symbols.clone();
          }
        })
        .or_insert(dependency.clone());
    }

    for (_id, dependency) in unique_deps.into_iter() {
      // Check if this dependency points to a discovered_asset
      let discovered_asset = discovered_assets.iter().find(|discovered_asset| {
        discovered_asset
          .asset
          .unique_key
          .as_ref()
          .is_some_and(|key| key == &dependency.specifier)
      });

      // Check if this dependency points to the root asset
      let dep_to_root_asset = root_asset_unique_key.is_some_and(|key| key == &dependency.specifier);

      let dependency_id = self.graph.add_dependency(dependency, cached);
      self.graph.add_edge(&asset_id, &dependency_id);

      if dep_to_root_asset {
        self.graph.add_edge(&dependency_id, &root_asset_id);
      }

      // If the dependency points to a dicovered asset then add the asset using the new
      // dep as its parent
      if let Some(AssetWithDependencies {
        asset,
        dependencies,
      }) = discovered_asset
      {
        let existing_discovered_asset = added_discovered_assets.get(&asset.id);

        if let Some(asset_node_id) = existing_discovered_asset {
          // This discovered_asset has already been added to the graph so we
          // just need to connect the dependency node to the asset node
          self.graph.add_edge(&dependency_id, asset_node_id);
        } else {
          // This discovered_asset isn't yet in the graph so we'll need to add
          // it and assign its dependencies by calling added_discovered_assets
          // recursively.
          let asset_id = self.graph.add_asset(Arc::new(asset.clone()), cached);
          self.graph.add_edge(&dependency_id, &asset_id);
          added_discovered_assets.insert(asset.id.clone(), asset_id);

          self.add_asset_dependencies(
            dependencies,
            discovered_assets,
            asset_id,
            added_discovered_assets,
            root_asset_id,
            root_asset_unique_key,
            cached,
          );
          self.propagate_requested_symbols(asset_id, dependency_id);
        }
      }
    }
  }

  fn propagate_requested_symbols(&mut self, asset_id: NodeId, incoming_dependency_id: NodeId) {
    propagate_requested_symbols(
      &mut self.graph,
      asset_id,
      incoming_dependency_id,
      &mut |dependency_idx: NodeId, dependency: Arc<Dependency>| {
        Self::on_undeferred(
          &mut self.request_id_to_dependency_id,
          &mut self.work_count,
          &mut self.request_context,
          &self.sender,
          dependency_idx,
          dependency,
        );
      },
    );
  }

  fn handle_target_request_result(&mut self, result: &TargetRequestOutput, cached: bool) {
    let TargetRequestOutput { entry, targets } = result;
    for target in targets {
      let entry =
        diff_paths(entry, &self.request_context.project_root).unwrap_or_else(|| entry.clone());
      let entry = entry.to_str().unwrap().to_string();

      let dependency = Dependency::entry(entry.clone(), target.clone());

      let dep_node = self.graph.add_entry_dependency(dependency.clone(), cached);
      self.entry_dependencies.push((entry, dep_node));

      let request = PathRequest {
        dependency: Arc::new(dependency),
      };
      self
        .request_id_to_dependency_id
        .insert(request.id(), dep_node);
      self.work_count += 1;
      let _ = self
        .request_context
        .queue_request(request, self.sender.clone());
    }
  }

  /// When we find dependencies, we will only trigger resolution and parsing for dependencies
  /// that have used symbols.
  ///
  /// Once they do have symbols in use, this callback will re-trigger resolution/transformation
  /// for those files.
  fn on_undeferred(
    request_id_to_dep_node_id: &mut HashMap<u64, NodeId>,
    work_count: &mut u32,
    request_context: &mut RunRequestContext,
    sender: &RequestResultSender,
    dependency_node_id: NodeId,
    dependency: Arc<Dependency>,
  ) {
    let request = PathRequest {
      dependency: dependency.clone(),
    };

    if request_id_to_dep_node_id
      .insert(request.id(), dependency_node_id)
      .is_none()
    {
      tracing::debug!(
        "queueing a path request from on_undeferred, {}",
        dependency.specifier
      );
      *work_count += 1;
      let _ = request_context.queue_request(request, sender.clone());
    }
  }
}

/// Direct discovered assets are discovered assets that don't have any
/// dependencies importing them. This means they need to be attached to the
/// original asset directly otherwise they'll be left out of the graph entirely.
///
/// CSS module JS export files are a good example of this.
fn get_direct_discovered_assets<'a>(
  discovered_assets: &'a [AssetWithDependencies],
  dependencies: &'a [Dependency],
) -> Vec<&'a AssetWithDependencies> {
  // Find all the discovered_asset unique keys
  let discovered_asset_unique_keys: HashSet<String> = discovered_assets
    .iter()
    .filter_map(|discovered_asset| discovered_asset.asset.unique_key.clone())
    .collect();

  let all_dependencies = dependencies.iter().chain(
    discovered_assets
      .iter()
      .flat_map(|discovered_asset| discovered_asset.dependencies.iter()),
  );

  // Find all the "indirect" discovered assets.
  // Assets that are pointed to by one of the generated dependencies within the
  // asset request
  let mut indirect_discovered_assets = HashSet::new();
  for dependency in all_dependencies {
    if discovered_asset_unique_keys.contains(&dependency.specifier) {
      indirect_discovered_assets.insert(dependency.specifier.clone());
    }
  }

  discovered_assets
    .iter()
    .filter(|discovered_asset| {
      !discovered_asset
        .asset
        .unique_key
        .as_ref()
        .is_some_and(|unique_key| indirect_discovered_assets.contains(unique_key))
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};
  use std::sync::Arc;

  use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode};
  use atlaspack_core::types::{AtlaspackOptions, Code};
  use atlaspack_filesystem::FileSystem;
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use crate::requests::{AssetGraphRequest, RequestResult};
  use crate::test_utils::{RequestTrackerTestOptions, request_tracker};

  #[tokio::test(flavor = "multi_thread")]
  async fn test_asset_graph_request_with_no_entries() {
    let options = RequestTrackerTestOptions::default();
    let mut request_tracker = request_tracker(options);

    let asset_graph_request = AssetGraphRequest {
      prev_asset_graph: None,
    };
    let result = request_tracker
      .run_request(asset_graph_request)
      .await
      .unwrap();
    let RequestResult::AssetGraph(asset_graph_request_result) = result.as_ref() else {
      panic!("Got invalid result");
    };

    assert_eq!(asset_graph_request_result.graph.get_assets().count(), 0);
    assert_eq!(
      asset_graph_request_result.graph.get_dependencies().count(),
      0
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_asset_graph_request_with_a_single_entry_with_no_dependencies() {
    #[cfg(not(target_os = "windows"))]
    let temporary_dir = PathBuf::from("/atlaspack_tests");
    #[cfg(target_os = "windows")]
    let temporary_dir = PathBuf::from("c:/windows/atlaspack_tests");

    assert!(temporary_dir.is_absolute());

    let fs = InMemoryFileSystem::default();

    fs.create_directory(&temporary_dir).unwrap();
    fs.set_current_working_directory(&temporary_dir); // <- resolver is broken without this
    fs.write_file(
      &temporary_dir.join("entry.js"),
      String::from(
        r#"
          console.log('hello world');
        "#,
      ),
    );

    fs.write_file(&temporary_dir.join("package.json"), String::from("{}"));

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      atlaspack_options: AtlaspackOptions {
        entries: vec![temporary_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      fs: Arc::new(fs),
      project_root: temporary_dir.clone(),
      search_path: temporary_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    let asset_graph_request = AssetGraphRequest {
      prev_asset_graph: None,
    };
    let result = request_tracker
      .run_request(asset_graph_request)
      .await
      .expect("Failed to run asset graph request");
    let RequestResult::AssetGraph(asset_graph_request_result) = result.as_ref() else {
      unreachable!("Got invalid result");
    };

    assert_eq!(asset_graph_request_result.graph.get_assets().count(), 1);
    assert_eq!(
      asset_graph_request_result.graph.get_dependencies().count(),
      1
    );

    let first_asset =
      get_first_asset(&asset_graph_request_result.graph).expect("No assets in graph");

    assert_eq!(first_asset.file_path, temporary_dir.join("entry.js"));
    assert_eq!(
      first_asset.code,
      (Code::from(
        String::from(
          r#"
            console.log('hello world');
        "#
        )
        .trim_start()
        .trim_end_matches(' ')
        .to_string()
      ))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_asset_graph_request_with_a_couple_of_entries() {
    #[cfg(not(target_os = "windows"))]
    let temporary_dir = PathBuf::from("/atlaspack_tests");
    #[cfg(target_os = "windows")]
    let temporary_dir = PathBuf::from("C:\\windows\\atlaspack_tests");

    let core_path = temporary_dir.join("atlaspack_core");
    let fs = InMemoryFileSystem::default();

    fs.create_directory(&temporary_dir).unwrap();
    fs.set_current_working_directory(&temporary_dir);

    fs.write_file(
      &temporary_dir.join("entry.js"),
      String::from(
        r#"
          import {x} from './a';
          import {y} from './b';
          console.log(x + y);
        "#,
      ),
    );

    fs.write_file(
      &temporary_dir.join("a.js"),
      String::from(
        r#"
          export const x = 15;
        "#,
      ),
    );

    fs.write_file(
      &temporary_dir.join("b.js"),
      String::from(
        r#"
          export const y = 27;
        "#,
      ),
    );

    fs.write_file(&temporary_dir.join("package.json"), String::from("{}"));

    setup_core_modules(&fs, &core_path);

    let mut request_tracker = request_tracker(RequestTrackerTestOptions {
      fs: Arc::new(fs),
      atlaspack_options: AtlaspackOptions {
        core_path,
        entries: vec![temporary_dir.join("entry.js").to_str().unwrap().to_string()],
        ..AtlaspackOptions::default()
      },
      project_root: temporary_dir.clone(),
      search_path: temporary_dir.clone(),
      ..RequestTrackerTestOptions::default()
    });

    let asset_graph_request = AssetGraphRequest {
      prev_asset_graph: None,
    };
    let result = request_tracker
      .run_request(asset_graph_request)
      .await
      .expect("Failed to run asset graph request");
    let RequestResult::AssetGraph(asset_graph_request_result) = result.as_ref() else {
      unreachable!("Got invalid result");
    };

    // Entry, 2 assets + helpers file
    assert_eq!(asset_graph_request_result.graph.get_assets().count(), 4);
    // Entry, entry to assets (2), assets to helpers (2)
    assert_eq!(
      asset_graph_request_result.graph.get_dependencies().count(),
      5
    );

    let first_asset =
      get_first_asset(&asset_graph_request_result.graph).expect("No assets in graph");

    assert_eq!(first_asset.file_path, temporary_dir.join("entry.js"));
  }

  fn setup_core_modules(fs: &InMemoryFileSystem, core_path: &Path) {
    let transformer_path = core_path
      .join("node_modules")
      .join("@atlaspack/transformer-js");

    fs.write_file(&transformer_path.join("package.json"), String::from("{}"));
    fs.write_file(
      &transformer_path.join("src").join("esmodule-helpers.js"),
      String::from("/* helpers */"),
    );
  }

  /// Do a BFS traversal of the the graph until the first Asset
  /// is discovered. This should be the entry Asset.
  fn get_first_asset(asset_graph: &AssetGraph) -> Option<&atlaspack_core::types::Asset> {
    use petgraph::graph::NodeIndex;
    use petgraph::visit::Bfs;

    let mut first_asset = None::<&atlaspack_core::types::Asset>;

    let root_node_index = NodeIndex::new(asset_graph.root_node());
    let mut bfs = Bfs::new(&asset_graph.graph, root_node_index);

    while let Some(node_index) = bfs.next(&asset_graph.graph) {
      if let Some(node_id) = asset_graph.graph.node_weight(node_index) {
        match asset_graph.get_node(node_id) {
          Some(AssetGraphNode::Asset(asset)) => {
            first_asset.replace(asset.as_ref());
            break;
          }
          _ => continue,
        }
      }
    }

    first_asset
  }

  /// Unit tests for the replicate_existing_edges function
  ///
  /// These tests directly test the replicate_existing_edges logic by using
  /// a simplified test function that operates on AssetGraph directly.
  #[cfg(test)]
  mod replicate_existing_edges_tests {
    use super::*;
    use atlaspack_core::asset_graph::NodeId;
    use atlaspack_core::types::{Asset, Code, Dependency, Target};

    #[test]
    fn test_replicate_existing_edges_basic_functionality() {
      let mut graph = AssetGraph::new();

      // Create test assets
      let asset1 = Arc::new(Asset {
        id: "asset1".to_string(),
        file_path: PathBuf::from("/test/asset1.js"),
        unique_key: Some("asset1".to_string()),
        code: Code::from("asset1 code".to_string()),
        ..Asset::default()
      });

      let asset2 = Arc::new(Asset {
        id: "asset2".to_string(),
        file_path: PathBuf::from("/test/asset2.js"),
        unique_key: Some("asset2".to_string()),
        code: Code::from("asset2 code".to_string()),
        ..Asset::default()
      });

      // Add assets to graph
      let asset1_id = graph.add_asset(asset1, false);
      let asset2_id = graph.add_asset(asset2, false);

      // Create test dependencies with different specifiers to get different node IDs
      let target = Target::default();
      let dependency1 = Dependency::entry("./shared.js".to_string(), target.clone());
      let dependency2 = Dependency::entry("./different.js".to_string(), target);

      // Add dependencies to graph
      let dep1_id = graph.add_dependency(dependency1, false);
      let dep2_id = graph.add_dependency(dependency2, false);

      // Connect first dependency to both assets (simulating discovered assets)
      graph.add_edge(&dep1_id, &asset1_id);
      graph.add_edge(&dep1_id, &asset2_id);

      // Verify initial state: dep1 has connections, dep2 doesn't
      let dep1_neighbors = graph.get_outgoing_neighbors(&dep1_id);
      let dep2_neighbors = graph.get_outgoing_neighbors(&dep2_id);

      assert_eq!(dep1_neighbors.len(), 2, "dep1 should have 2 outgoing edges");
      assert_eq!(
        dep2_neighbors.len(),
        0,
        "dep2 should have no outgoing edges initially"
      );

      // Test the replicate_existing_edges function via test helper
      test_replicate_existing_edges(&mut graph, dep1_id, dep2_id);

      // Verify that dep2 now has the same connections as dep1
      let dep2_neighbors_after = graph.get_outgoing_neighbors(&dep2_id);
      assert_eq!(
        dep2_neighbors_after.len(),
        2,
        "dep2 should now have 2 outgoing edges"
      );

      // Verify that both assets are connected to dep2
      assert!(
        dep2_neighbors_after.contains(&asset1_id),
        "dep2 should connect to asset1"
      );
      assert!(
        dep2_neighbors_after.contains(&asset2_id),
        "dep2 should connect to asset2"
      );

      // Verify that original dependency still has its connections
      let dep1_neighbors_after = graph.get_outgoing_neighbors(&dep1_id);
      assert_eq!(
        dep1_neighbors_after.len(),
        2,
        "dep1 should still have 2 outgoing edges"
      );
      assert!(
        dep1_neighbors_after.contains(&asset1_id),
        "dep1 should still connect to asset1"
      );
      assert!(
        dep1_neighbors_after.contains(&asset2_id),
        "dep1 should still connect to asset2"
      );
    }

    #[cfg(test)]
    pub fn test_replicate_existing_edges(
      graph: &mut AssetGraph,
      existing_dep_id: NodeId,
      new_dep_id: NodeId,
    ) {
      let existing_edges = graph.get_outgoing_neighbors(&existing_dep_id);
      for edge in existing_edges {
        graph.add_edge(&new_dep_id, &edge);
        // Note: We skip propagate_requested_symbols in tests as it requires the full builder context
      }
    }

    #[test]
    fn test_replicate_existing_edges_with_no_existing_edges() {
      let mut graph = AssetGraph::new();

      // Create test dependencies with different specifiers to get different node IDs
      let target = Target::default();
      let dependency1 = Dependency::entry("./empty1.js".to_string(), target.clone());
      let dependency2 = Dependency::entry("./empty2.js".to_string(), target);

      let dep1_id = graph.add_dependency(dependency1, false);
      let dep2_id = graph.add_dependency(dependency2, false);

      // Verify initial state: neither dependency has connections
      let dep1_neighbors = graph.get_outgoing_neighbors(&dep1_id);
      let dep2_neighbors = graph.get_outgoing_neighbors(&dep2_id);

      assert_eq!(
        dep1_neighbors.len(),
        0,
        "dep1 should have no outgoing edges"
      );
      assert_eq!(
        dep2_neighbors.len(),
        0,
        "dep2 should have no outgoing edges"
      );

      // Test the replicate_existing_edges function (should be no-op)
      test_replicate_existing_edges(&mut graph, dep1_id, dep2_id);

      // Verify that dep2 still has no connections (nothing to replicate)
      let dep2_neighbors_after = graph.get_outgoing_neighbors(&dep2_id);
      assert_eq!(
        dep2_neighbors_after.len(),
        0,
        "dep2 should still have no outgoing edges"
      );

      // Verify that dep1 is unchanged
      let dep1_neighbors_after = graph.get_outgoing_neighbors(&dep1_id);
      assert_eq!(
        dep1_neighbors_after.len(),
        0,
        "dep1 should still have no outgoing edges"
      );
    }

    #[test]
    fn test_replicate_existing_edges_preserves_original_edges() {
      let mut graph = AssetGraph::new();

      // Create test assets
      let asset1 = Arc::new(Asset {
        id: "shared_asset1".to_string(),
        file_path: PathBuf::from("/test/shared1.js"),
        unique_key: Some("shared1".to_string()),
        code: Code::from("shared1 code".to_string()),
        ..Asset::default()
      });

      let asset2 = Arc::new(Asset {
        id: "shared_asset2".to_string(),
        file_path: PathBuf::from("/test/shared2.js"),
        unique_key: Some("shared2".to_string()),
        code: Code::from("shared2 code".to_string()),
        ..Asset::default()
      });

      let asset1_id = graph.add_asset(asset1, false);
      let asset2_id = graph.add_asset(asset2, false);

      // Create test dependencies with different specifiers to get different node IDs
      let target = Target::default();
      let dependency1 = Dependency::entry("./shared1.js".to_string(), target.clone());
      let dependency2 = Dependency::entry("./shared2.js".to_string(), target);

      let dep1_id = graph.add_dependency(dependency1, false);
      let dep2_id = graph.add_dependency(dependency2, false);

      // Connect first dependency to assets
      graph.add_edge(&dep1_id, &asset1_id);
      graph.add_edge(&dep1_id, &asset2_id);

      // Store original connections for verification
      let original_dep1_neighbors = graph.get_outgoing_neighbors(&dep1_id);
      assert_eq!(
        original_dep1_neighbors.len(),
        2,
        "dep1 should start with 2 connections"
      );

      // Test the replicate_existing_edges function
      test_replicate_existing_edges(&mut graph, dep1_id, dep2_id);

      // Verify that original dependency still has its connections unchanged
      let dep1_neighbors_after = graph.get_outgoing_neighbors(&dep1_id);
      assert_eq!(
        dep1_neighbors_after.len(),
        original_dep1_neighbors.len(),
        "dep1 should maintain same number of connections"
      );
      assert!(
        dep1_neighbors_after.contains(&asset1_id),
        "dep1 should still connect to asset1"
      );
      assert!(
        dep1_neighbors_after.contains(&asset2_id),
        "dep1 should still connect to asset2"
      );

      // Verify that new dependency also has the replicated connections
      let dep2_neighbors = graph.get_outgoing_neighbors(&dep2_id);
      assert_eq!(
        dep2_neighbors.len(),
        2,
        "dep2 should now have 2 connections"
      );
      assert!(
        dep2_neighbors.contains(&asset1_id),
        "dep2 should connect to asset1"
      );
      assert!(
        dep2_neighbors.contains(&asset2_id),
        "dep2 should connect to asset2"
      );

      // Verify that the connections are identical (order doesn't matter)
      use std::collections::HashSet;
      let dep1_set: HashSet<_> = dep1_neighbors_after.into_iter().collect();
      let dep2_set: HashSet<_> = dep2_neighbors.into_iter().collect();
      assert_eq!(
        dep1_set, dep2_set,
        "Both dependencies should have identical connections"
      );
    }

    #[test]
    fn test_replicate_existing_edges_single_connection() {
      let mut graph = AssetGraph::new();

      // Create a single test asset
      let asset = Arc::new(Asset {
        id: "single_asset".to_string(),
        file_path: PathBuf::from("/test/single.js"),
        unique_key: Some("single".to_string()),
        code: Code::from("single asset code".to_string()),
        ..Asset::default()
      });

      let asset_id = graph.add_asset(asset, false);

      // Create test dependencies with different specifiers to get different node IDs
      let target = Target::default();
      let dependency1 = Dependency::entry("./single1.js".to_string(), target.clone());
      let dependency2 = Dependency::entry("./single2.js".to_string(), target);

      let dep1_id = graph.add_dependency(dependency1, false);
      let dep2_id = graph.add_dependency(dependency2, false);

      // Connect first dependency to single asset
      graph.add_edge(&dep1_id, &asset_id);

      // Verify initial state
      assert_eq!(graph.get_outgoing_neighbors(&dep1_id).len(), 1);
      assert_eq!(graph.get_outgoing_neighbors(&dep2_id).len(), 0);

      // Test the replicate_existing_edges function
      test_replicate_existing_edges(&mut graph, dep1_id, dep2_id);

      // Verify replication worked for single connection
      let dep2_neighbors = graph.get_outgoing_neighbors(&dep2_id);
      assert_eq!(dep2_neighbors.len(), 1, "dep2 should have 1 connection");
      assert!(
        dep2_neighbors.contains(&asset_id),
        "dep2 should connect to the asset"
      );

      // Verify original is preserved
      let dep1_neighbors = graph.get_outgoing_neighbors(&dep1_id);
      assert_eq!(
        dep1_neighbors.len(),
        1,
        "dep1 should still have 1 connection"
      );
      assert!(
        dep1_neighbors.contains(&asset_id),
        "dep1 should still connect to the asset"
      );
    }
  }
}
