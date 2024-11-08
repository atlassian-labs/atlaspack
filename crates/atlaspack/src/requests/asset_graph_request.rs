use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use indexmap::IndexMap;
use pathdiff::diff_paths;
use petgraph::graph::NodeIndex;

use atlaspack_core::asset_graph::{AssetGraph, DependencyNode, DependencyState};
use atlaspack_core::types::{Asset, AssetWithDependencies, Dependency};

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::asset_request::{AssetRequest, AssetRequestOutput};
use super::entry_request::{EntryRequest, EntryRequestOutput};
use super::path_request::{PathRequest, PathRequestOutput};
use super::target_request::{TargetRequest, TargetRequestOutput};
use super::RequestResult;

type ResultSender = Sender<Result<(RequestResult, u64), anyhow::Error>>;
type ResultReceiver = Receiver<Result<(RequestResult, u64), anyhow::Error>>;

/// The AssetGraphRequest is in charge of building the AssetGraphRequest
/// In doing so, it kicks of the EntryRequest, TargetRequest, PathRequest and AssetRequests.
#[derive(Debug, Hash)]
pub struct AssetGraphRequest {}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetGraphRequestOutput {
  pub graph: AssetGraph,
}

#[async_trait]
impl Request for AssetGraphRequest {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let builder = AssetGraphBuilder::new(request_context);

    builder.build()
  }
}

struct AssetGraphBuilder {
  request_id_to_dep_node_index: HashMap<u64, NodeIndex>,
  graph: AssetGraph,
  visited: HashSet<u64>,
  work_count: u32,
  request_context: RunRequestContext,
  sender: ResultSender,
  receiver: ResultReceiver,
  asset_request_to_asset: HashMap<u64, NodeIndex>,
  waiting_asset_requests: HashMap<u64, HashSet<NodeIndex>>,
}

impl AssetGraphBuilder {
  fn new(request_context: RunRequestContext) -> Self {
    let (sender, receiver) = channel();

    AssetGraphBuilder {
      request_id_to_dep_node_index: HashMap::new(),
      graph: AssetGraph::new(),
      visited: HashSet::new(),
      work_count: 0,
      request_context,
      sender,
      receiver,
      asset_request_to_asset: HashMap::new(),
      waiting_asset_requests: HashMap::new(),
    }
  }

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

      match result {
        Ok((RequestResult::Entry(result), _request_id)) => {
          tracing::debug!("Handling EntryRequestOutput");
          self.handle_entry_result(result);
        }
        Ok((RequestResult::Target(result), _request_id)) => {
          tracing::debug!("Handling TargetRequestOutput");
          self.handle_target_request_result(result);
        }
        Ok((RequestResult::Asset(result), request_id)) => {
          tracing::debug!(
            "Handling AssetRequestOutput: {}",
            result.asset.file_path.display()
          );
          self.handle_asset_result(result, request_id);
        }
        Ok((RequestResult::Path(result), request_id)) => {
          tracing::debug!("Handling PathRequestOutput");
          self.handle_path_result(result, request_id);
        }
        Err(err) => return Err(err),
        // This branch should never occur
        Ok((result, request_id)) => {
          return Err(anyhow!(
            "Unexpected request result in AssetGraphRequest ({}): {:?}",
            request_id,
            result
          ))
        }
      }
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::AssetGraph(AssetGraphRequestOutput { graph: self.graph }),
      invalidations: vec![],
    })
  }

  fn handle_path_result(&mut self, result: PathRequestOutput, request_id: u64) {
    let node = *self
      .request_id_to_dep_node_index
      .get(&request_id)
      .expect("Missing node index for request id {request_id}");
    let dep_index = self.graph.dependency_index(node).unwrap();
    let DependencyNode {
      dependency,
      requested_symbols,
      state,
    } = &mut self.graph.dependencies[dep_index];
    let asset_request = match result {
      PathRequestOutput::Resolved {
        path,
        code,
        pipeline,
        side_effects,
        query,
        can_defer,
      } => {
        if !side_effects
          && can_defer
          && requested_symbols.is_empty()
          && dependency.symbols.is_some()
        {
          *state = DependencyState::Deferred;
          return;
        }

        *state = DependencyState::Resolved;
        AssetRequest {
          code: code.clone(),
          env: dependency.env.clone().into(),
          file_path: path,
          project_root: self.request_context.project_root.clone(),
          pipeline: pipeline.clone(),
          query,
          side_effects,
        }
      }
      PathRequestOutput::Excluded => {
        *state = DependencyState::Excluded;
        return;
      }
    };
    let id = asset_request.id();

    if self.visited.insert(id) {
      self.request_id_to_dep_node_index.insert(id, node);
      self.work_count += 1;
      let _ = self
        .request_context
        .queue_request(asset_request, self.sender.clone());
    } else if let Some(asset_node_index) = self.asset_request_to_asset.get(&id) {
      // We have already completed this AssetRequest so we can connect the
      // Dependency to the Asset immediately
      self.graph.add_edge(&node, asset_node_index);
      self.propagate_requested_symbols(*asset_node_index, node);
    } else {
      // The AssetRequest has already been kicked off but is yet to
      // complete. Register this Dependency to be connected once it
      // completes
      self
        .waiting_asset_requests
        .entry(id)
        .and_modify(|nodes| {
          nodes.insert(node);
        })
        .or_insert_with(|| HashSet::from([node]));
    }
  }

  fn handle_entry_result(&mut self, result: EntryRequestOutput) {
    let EntryRequestOutput { entries } = result;
    for entry in entries {
      let target_request = TargetRequest {
        default_target_options: self.request_context.options.default_target_options.clone(),
        entry,
        env: self.request_context.options.env.clone(),
        mode: self.request_context.options.mode.clone(),
      };

      self.work_count += 1;
      let _ = self
        .request_context
        .queue_request(target_request, self.sender.clone());
    }
  }

  fn handle_asset_result(&mut self, result: AssetRequestOutput, request_id: u64) {
    let AssetRequestOutput {
      asset,
      discovered_assets,
      dependencies,
    } = result;
    let incoming_dep_node_index = *self
      .request_id_to_dep_node_index
      .get(&request_id)
      .expect("Missing node index for request id {request_id}");

    // Connect the incoming DependencyNode to the new AssetNode
    let asset_node_index = self.graph.add_asset(incoming_dep_node_index, asset.clone());

    self
      .asset_request_to_asset
      .insert(request_id, asset_node_index);

    let root_asset = (&asset, asset_node_index);
    let mut added_discovered_assets: HashMap<String, NodeIndex> = HashMap::new();

    // Attach the "direct" discovered assets to the graph
    let direct_discovered_assets = get_direct_discovered_assets(&discovered_assets, &dependencies);
    for discovered_asset in direct_discovered_assets {
      let asset_node_index = self
        .graph
        .add_asset(incoming_dep_node_index, discovered_asset.asset.clone());

      self.add_asset_dependencies(
        &discovered_asset.dependencies,
        &discovered_assets,
        asset_node_index,
        &mut added_discovered_assets,
        root_asset,
      );
      self.propagate_requested_symbols(asset_node_index, incoming_dep_node_index);
    }

    self.add_asset_dependencies(
      &dependencies,
      &discovered_assets,
      asset_node_index,
      &mut added_discovered_assets,
      root_asset,
    );

    self.propagate_requested_symbols(asset_node_index, incoming_dep_node_index);

    // Connect any previously discovered Dependencies that were waiting
    // for this AssetNode to be created
    if let Some(waiting) = self.waiting_asset_requests.remove(&request_id) {
      for dep in waiting {
        self.graph.add_edge(&dep, &asset_node_index);
        self.propagate_requested_symbols(asset_node_index, dep);
      }
    }
  }

  fn add_asset_dependencies(
    &mut self,
    dependencies: &Vec<Dependency>,
    discovered_assets: &Vec<AssetWithDependencies>,
    asset_node_index: NodeIndex,
    added_discovered_assets: &mut HashMap<String, NodeIndex>,
    root_asset: (&Asset, NodeIndex),
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
      let dep_to_root_asset = root_asset
        .0
        .unique_key
        .as_ref()
        .is_some_and(|key| key == &dependency.specifier);

      let dep_node = self.graph.add_dependency(asset_node_index, dependency);

      if dep_to_root_asset {
        self.graph.add_edge(&dep_node, &root_asset.1);
      }

      // If the dependency points to a dicovered asset then add the asset using the new
      // dep as it's parent
      if let Some(AssetWithDependencies {
        asset,
        dependencies,
      }) = discovered_asset
      {
        let existing_discovered_asset = added_discovered_assets.get(&asset.id);

        if let Some(asset_node_index) = existing_discovered_asset {
          // This discovered_asset has already been added to the graph so we
          // just need to connect the dependency node to the asset node
          self.graph.add_edge(&dep_node, asset_node_index);
        } else {
          // This discovered_asset isn't yet in the graph so we'll need to add
          // it and assign it's dependencies by calling added_discovered_assets
          // recursively.
          let asset_node_index = self.graph.add_asset(dep_node, asset.clone());
          added_discovered_assets.insert(asset.id.clone(), asset_node_index);

          self.add_asset_dependencies(
            dependencies,
            discovered_assets,
            asset_node_index,
            added_discovered_assets,
            root_asset,
          );
          self.propagate_requested_symbols(asset_node_index, dep_node);
        }
      }
    }
  }

  fn propagate_requested_symbols(
    &mut self,
    asset_node_index: NodeIndex,
    incoming_dep_node_index: NodeIndex,
  ) {
    self.graph.propagate_requested_symbols(
      asset_node_index,
      incoming_dep_node_index,
      &mut |dependency_node_index: NodeIndex, dependency: Arc<Dependency>| {
        Self::on_undeferred(
          &mut self.request_id_to_dep_node_index,
          &mut self.work_count,
          &mut self.request_context,
          &self.sender,
          dependency_node_index,
          dependency,
        );
      },
    );
  }

  fn handle_target_request_result(&mut self, result: TargetRequestOutput) {
    let TargetRequestOutput { entry, targets } = result;
    for target in targets {
      let entry =
        diff_paths(&entry, &self.request_context.project_root).unwrap_or_else(|| entry.clone());

      let dependency = Dependency::entry(entry.to_str().unwrap().to_string(), target);

      let dep_node = self.graph.add_entry_dependency(dependency.clone());

      let request = PathRequest {
        dependency: Arc::new(dependency),
      };
      self
        .request_id_to_dep_node_index
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
    request_id_to_dep_node_index: &mut HashMap<u64, NodeIndex>,
    work_count: &mut u32,
    request_context: &mut RunRequestContext,
    sender: &ResultSender,
    dependency_node_index: NodeIndex,
    dependency: Arc<Dependency>,
  ) {
    let request = PathRequest {
      dependency: dependency.clone(),
    };

    request_id_to_dep_node_index.insert(request.id(), dependency_node_index);
    tracing::debug!(
      "queueing a path request from on_undeferred, {}",
      dependency.specifier
    );
    *work_count += 1;
    let _ = request_context.queue_request(request, sender.clone());
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

  use atlaspack_core::types::{AtlaspackOptions, Code};
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
  use atlaspack_filesystem::FileSystem;

  use crate::requests::{AssetGraphRequest, RequestResult};
  use crate::test_utils::{request_tracker, RequestTrackerTestOptions};

  #[tokio::test(flavor = "multi_thread")]
  async fn test_asset_graph_request_with_no_entries() {
    let options = RequestTrackerTestOptions::default();
    let mut request_tracker = request_tracker(options);

    let asset_graph_request = AssetGraphRequest {};
    let RequestResult::AssetGraph(asset_graph_request_result) = request_tracker
      .run_request(asset_graph_request)
      .await
      .unwrap()
    else {
      assert!(false, "Got invalid result");
      return;
    };

    assert_eq!(asset_graph_request_result.graph.assets.len(), 0);
    assert_eq!(asset_graph_request_result.graph.dependencies.len(), 0);
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

    let asset_graph_request = AssetGraphRequest {};
    let RequestResult::AssetGraph(asset_graph_request_result) = request_tracker
      .run_request(asset_graph_request)
      .await
      .expect("Failed to run asset graph request")
    else {
      assert!(false, "Got invalid result");
      return;
    };

    assert_eq!(asset_graph_request_result.graph.assets.len(), 1);
    assert_eq!(asset_graph_request_result.graph.dependencies.len(), 1);
    assert_eq!(
      asset_graph_request_result
        .graph
        .assets
        .get(0)
        .unwrap()
        .asset
        .file_path,
      temporary_dir.join("entry.js")
    );
    assert_eq!(
      asset_graph_request_result
        .graph
        .assets
        .get(0)
        .unwrap()
        .asset
        .code,
      (Code::from(
        String::from(
          r#"
            console.log('hello world');
        "#
        )
        .trim_start()
        .trim_end_matches(|p| p == ' ')
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

    let asset_graph_request = AssetGraphRequest {};
    let RequestResult::AssetGraph(asset_graph_request_result) = request_tracker
      .run_request(asset_graph_request)
      .await
      .expect("Failed to run asset graph request")
    else {
      assert!(false, "Got invalid result");
      return;
    };

    // Entry, 2 assets + helpers file
    assert_eq!(asset_graph_request_result.graph.assets.len(), 4);
    // Entry, entry to assets (2), assets to helpers (2)
    assert_eq!(asset_graph_request_result.graph.dependencies.len(), 5);

    assert_eq!(
      asset_graph_request_result
        .graph
        .assets
        .get(0)
        .unwrap()
        .asset
        .file_path,
      temporary_dir.join("entry.js")
    );
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
}
