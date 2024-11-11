use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ::futures::future::BoxFuture;
use ::futures::FutureExt;
use anyhow::anyhow;
use async_trait::async_trait;
use indexmap::IndexMap;
use pathdiff::diff_paths;
use petgraph::graph::NodeIndex;
use tokio::sync::futures;
use tokio::sync::mpsc::{Receiver, Sender};

use atlaspack_core::asset_graph::{AssetGraph, DependencyNode};
use atlaspack_core::types::{Asset, AssetWithDependencies, Dependency};
use tokio::task::spawn_blocking;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::asset_request::{AssetRequest, AssetRequestOutput};
use super::entry_request::{Entry, EntryRequest, EntryRequestOutput};
use super::path_request::{PathRequest, PathRequestOutput};
use super::target_request::{TargetRequest, TargetRequestOutput};
use super::RequestResult;

type AssetGraphChildResult = Result<(RequestResult, u64), anyhow::Error>;
type ResultSender = Sender<AssetGraphChildResult>;
type ResultReceiver = Receiver<AssetGraphChildResult>;

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

    builder.build().await
  }
}

struct AssetGraphBuilder {
  request_id_to_dep_node_index: HashMap<u64, NodeIndex>,
  graph: AssetGraph,
  visited: HashSet<u64>,
  work_count: u32,
  request_context: Arc<RunRequestContext>,
  asset_request_to_asset: HashMap<u64, NodeIndex>,
  waiting_asset_requests: HashMap<u64, HashSet<NodeIndex>>,
}

impl AssetGraphBuilder {
  fn new(request_context: RunRequestContext) -> Self {
    AssetGraphBuilder {
      request_id_to_dep_node_index: HashMap::new(),
      graph: AssetGraph::new(),
      visited: HashSet::new(),
      work_count: 0,
      request_context: Arc::new(request_context),
      asset_request_to_asset: HashMap::new(),
      waiting_asset_requests: HashMap::new(),
    }
  }

  async fn build(self) -> Result<ResultAndInvalidations, RunRequestError> {
    let mut futures = vec![];
    for entry in &self.request_context.options.entries {
      let f = tokio::spawn({
        let request_context = self.request_context.clone();
        let entry = entry.clone();
        async move { run_entry_request(&request_context, &entry).await }
      });
      futures.push(f);
    }

    for entry_future in futures {
      entry_future.await??;
    }

    tracing::info!("Finished building asset graph!");

    Ok(ResultAndInvalidations::new(
      RequestResult::AssetGraph(AssetGraphRequestOutput { graph: self.graph }),
      vec![],
    ))
  }

  // async fn handle_path_result(
  //   &mut self,
  //   result: PathRequestOutput,
  //   request_id: u64,
  // ) -> anyhow::Result<()> {
  //   let node = *self
  //     .request_id_to_dep_node_index
  //     .get(&request_id)
  //     .expect("Missing node index for request id {request_id}");
  //   let dep_index = self.graph.dependency_index(node).unwrap();
  //   let DependencyNode { dependency, .. } = &mut self.graph.dependencies[dep_index];
  //   let asset_request = match result {
  //     PathRequestOutput::Resolved {
  //       path,
  //       code,
  //       pipeline,
  //       side_effects,
  //       query,
  //     } => AssetRequest {
  //       code: code.clone(),
  //       env: dependency.env.clone(),
  //       file_path: path,
  //       project_root: self.request_context.project_root.clone(),
  //       pipeline: pipeline.clone(),
  //       query,
  //       side_effects,
  //     },
  //     PathRequestOutput::Excluded => {
  //       return Ok(());
  //     }
  //   };
  //   let id = asset_request.id();

  //   if self.visited.insert(id) {
  //     self.request_id_to_dep_node_index.insert(id, node);
  //     self.work_count += 1;
  //     self.request_context.run_request(asset_request).await?;
  //   } else if let Some(asset_node_index) = self.asset_request_to_asset.get(&id) {
  //     // We have already completed this AssetRequest so we can connect the
  //     // Dependency to the Asset immediately
  //     self.graph.add_edge(&node, asset_node_index);
  //   } else {
  //     // The AssetRequest has already been kicked off but is yet to
  //     // complete. Register this Dependency to be connected once it
  //     // completes
  //     self
  //       .waiting_asset_requests
  //       .entry(id)
  //       .and_modify(|nodes| {
  //         nodes.insert(node);
  //       })
  //       .or_insert_with(|| HashSet::from([node]));
  //   }

  //   Ok(())
  // }

  // async fn handle_asset_result(
  //   &mut self,
  //   result: AssetRequestOutput,
  //   request_id: u64,
  // ) -> anyhow::Result<()> {
  //   let AssetRequestOutput {
  //     asset,
  //     discovered_assets,
  //     dependencies,
  //   } = result;
  //   let incoming_dep_node_index = *self
  //     .request_id_to_dep_node_index
  //     .get(&request_id)
  //     .expect("Missing node index for request id {request_id}");

  //   // Connect the incoming DependencyNode to the new AssetNode
  //   let asset_node_index = self.graph.add_asset(incoming_dep_node_index, asset.clone());

  //   self
  //     .asset_request_to_asset
  //     .insert(request_id, asset_node_index);

  //   let root_asset = (&asset, asset_node_index);
  //   let mut added_discovered_assets: HashMap<String, NodeIndex> = HashMap::new();

  //   // Attach the "direct" discovered assets to the graph
  //   let direct_discovered_assets = get_direct_discovered_assets(&discovered_assets, &dependencies);
  //   for discovered_asset in direct_discovered_assets {
  //     let asset_node_index = self
  //       .graph
  //       .add_asset(incoming_dep_node_index, discovered_asset.asset.clone());

  //     self
  //       .add_asset_dependencies(
  //         &discovered_asset.dependencies,
  //         &discovered_assets,
  //         asset_node_index,
  //         &mut added_discovered_assets,
  //         root_asset,
  //       )
  //       .await?;
  //   }

  //   self
  //     .add_asset_dependencies(
  //       &dependencies,
  //       &discovered_assets,
  //       asset_node_index,
  //       &mut added_discovered_assets,
  //       root_asset,
  //     )
  //     .await?;

  //   // Connect any previously discovered Dependencies that were waiting
  //   // for this AssetNode to be created
  //   if let Some(waiting) = self.waiting_asset_requests.remove(&request_id) {
  //     for dep in waiting {
  //       self.graph.add_edge(&dep, &asset_node_index);
  //     }
  //   }

  //   Ok(())
  // }

  // async fn add_asset_dependencies(
  //   &mut self,
  //   dependencies: &Vec<Arc<Dependency>>,
  //   discovered_assets: &Vec<AssetWithDependencies>,
  //   asset_node_index: NodeIndex,
  //   added_discovered_assets: &mut HashMap<String, NodeIndex>,
  //   root_asset: (&Asset, NodeIndex),
  // ) -> anyhow::Result<()> {
  //   struct Item<'a> {
  //     asset_node_index: NodeIndex,
  //     dependencies: &'a Vec<Arc<Dependency>>,
  //   }

  //   let mut queue: VecDeque<_> = VecDeque::new();
  //   queue.push_back(Item {
  //     asset_node_index,
  //     dependencies,
  //   });
  //   while let Some(Item {
  //     asset_node_index,
  //     dependencies,
  //   }) = queue.pop_front()
  //   {
  //     // Connect dependencies of the Asset
  //     let mut unique_deps: IndexMap<String, Dependency> = IndexMap::new();
  //     for dependency in dependencies {
  //       unique_deps
  //         .entry(dependency.id())
  //         .and_modify(|d| {
  //           // This code is an incomplete version of mergeDependencies in packages/core/core/src/Dependency.js
  //           // Duplicate dependencies can occur when node globals are polyfilled
  //           // e.g. 'process'. I think ideally we wouldn't end up with two
  //           // dependencies post-transform but that needs further investigation to
  //           // resolve and understand...
  //           d.meta.extend(dependency.meta.clone());
  //           if let Some(symbols) = d.symbols.as_mut() {
  //             if let Some(merge_symbols) = dependency.symbols.as_ref() {
  //               symbols.extend(merge_symbols.clone());
  //             }
  //           } else {
  //             d.symbols = dependency.symbols.clone();
  //           }
  //         })
  //         .or_insert(dependency.clone());
  //     }
  //     for (_id, dependency) in unique_deps.into_iter() {
  //       tracing::debug!("Adding dependency to asset {}", dependency.specifier);
  //       // Check if this dependency points to a discovered_asset
  //       let discovered_asset = discovered_assets.iter().find(|discovered_asset| {
  //         discovered_asset
  //           .asset
  //           .unique_key
  //           .as_ref()
  //           .is_some_and(|key| key == &dependency.specifier)
  //       });
  //       // Check if this dependency points to the root asset
  //       let dep_to_root_asset = root_asset
  //         .0
  //         .unique_key
  //         .as_ref()
  //         .is_some_and(|key| key == &dependency.specifier);
  //       let dependency = Arc::new(dependency);
  //       let dep_node = self
  //         .graph
  //         .add_dependency(asset_node_index, dependency.clone());
  //       Self::trigger_path_request(
  //         &mut self.request_id_to_dep_node_index,
  //         &mut self.work_count,
  //         &self.request_context,
  //         dep_node,
  //         dependency,
  //       )
  //       .await?;
  //       if dep_to_root_asset {
  //         self.graph.add_edge(&dep_node, &root_asset.1);
  //       }
  //       // If the dependency points to a dicovered asset then add the asset using the new
  //       // dep as it's parent
  //       if let Some(AssetWithDependencies {
  //         asset,
  //         dependencies,
  //       }) = discovered_asset
  //       {
  //         let existing_discovered_asset = added_discovered_assets.get(&asset.id);
  //         if let Some(asset_node_index) = existing_discovered_asset {
  //           // This discovered_asset has already been added to the graph so we
  //           // just need to connect the dependency node to the asset node
  //           self.graph.add_edge(&dep_node, asset_node_index);
  //         } else {
  //           // This discovered_asset isn't yet in the graph so we'll need to add
  //           // it and assign it's dependencies by calling added_discovered_assets
  //           // recursively.
  //           let asset_node_index = self.graph.add_asset(dep_node, asset.clone());
  //           added_discovered_assets.insert(asset.id.clone(), asset_node_index);
  //           queue.push_back(Item {
  //             dependencies,
  //             asset_node_index,
  //           });
  //         }
  //       }
  //     }
  //   }
  //   Ok(())
  // }

  async fn handle_target_request_result(
    &mut self,
    result: TargetRequestOutput,
  ) -> anyhow::Result<()> {
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
      self.request_context.run_request(request).await?;
    }

    Ok(())
  }

  async fn trigger_path_request(
    request_id_to_dep_node_index: &mut HashMap<u64, NodeIndex>,
    work_count: &mut u32,
    request_context: &RunRequestContext,
    dependency_node_index: NodeIndex,
    dependency: Arc<Dependency>,
  ) -> anyhow::Result<()> {
    let request = PathRequest {
      dependency: dependency.clone(),
    };

    if request_id_to_dep_node_index.contains_key(&request.id()) {
      return Ok(());
    }

    request_id_to_dep_node_index.insert(request.id(), dependency_node_index);
    // tracing::debug!(
    //   "queueing a path request from on_undeferred, {:#?}",
    //   dependency
    // );
    *work_count += 1;
    request_context.run_request(request).await?;

    Ok(())
  }
}

/// Direct discovered assets are discovered assets that don't have any
/// dependencies importing them. This means they need to be attached to the
/// original asset directly otherwise they'll be left out of the graph entirely.
///
/// CSS module JS export files are a good example of this.
fn get_direct_discovered_assets<'a>(
  discovered_assets: &'a [AssetWithDependencies],
  dependencies: &'a [Arc<Dependency>],
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

#[tracing::instrument(skip(request_context, entry))]
async fn run_entry_request(
  request_context: &Arc<RunRequestContext>,
  entry: &str,
) -> anyhow::Result<()> {
  let entry_result = request_context
    .run_request(EntryRequest {
      entry: entry.to_string(),
    })
    .await?;
  let RequestResult::Entry(EntryRequestOutput { entries }) = &*entry_result else {
    return Err(anyhow!("Failed to run entry request"));
  };

  let mut futures = vec![];
  for entry in entries.iter().cloned() {
    let future = tokio::spawn({
      let request_context = request_context.clone();
      async move { run_target_request(request_context, entry).await }
    });
    futures.push(future);
  }

  for future in futures {
    future.await??;
  }

  Ok(())
}

async fn run_target_request(
  request_context: Arc<RunRequestContext>,
  entry: Entry,
) -> anyhow::Result<()> {
  let target_request = TargetRequest {
    default_target_options: request_context.options.default_target_options.clone(),
    entry: entry,
    env: request_context.options.env.clone(),
    mode: request_context.options.mode.clone(),
  };
  let result = request_context.run_request(target_request).await?;
  let TargetRequestOutput { entry, targets } = result.as_target().unwrap();

  let mut futures = vec![];
  for target in targets.iter().cloned() {
    let entry = diff_paths(&entry, &request_context.project_root).unwrap_or_else(|| entry.clone());
    let dependency = Dependency::entry(entry.to_str().unwrap().to_string(), target);
    let dependency = Arc::new(dependency);

    // let dep_node = self.graph.add_entry_dependency(dependency.clone());

    let request_context = request_context.clone();
    let future = async move { run_path_request(request_context, dependency).await };
    futures.push(tokio::spawn(future));

    // let request = PathRequest {
    //   dependency: Arc::new(dependency),
    // };
    // self
    //   .request_id_to_dep_node_index
    //   .insert(request.id(), dep_node);
    // self.work_count += 1;
    // request_context.run_request(request).await?;
  }

  for future in futures {
    future.await??;
  }

  Ok(())
}

fn run_path_request(
  request_context: Arc<RunRequestContext>,
  dependency: Arc<Dependency>,
) -> BoxFuture<'static, anyhow::Result<()>> {
  // We need to box this due to recursion
  async move {
    let request = PathRequest {
      dependency: dependency.clone(),
    };
    let result = request_context.run_request(request).await?;

    let PathRequestOutput::Resolved {
      path,
      code,
      pipeline,
      side_effects,
      query,
    } = result.as_path().unwrap()
    else {
      // excluded path results just move on
      return Ok(());
    };

    let asset_request = AssetRequest {
      code: code.clone(),
      env: dependency.env.clone(),
      file_path: path.clone(),
      project_root: request_context.project_root.clone(),
      pipeline: pipeline.clone(),
      query: query.clone(),
      side_effects: *side_effects,
    };
    run_asset_request(request_context, asset_request).await?;

    Ok(())
  }
  .boxed()
}

async fn run_asset_request(
  request_context: Arc<RunRequestContext>,
  asset_request: AssetRequest,
) -> anyhow::Result<()> {
  let result = request_context.run_request(asset_request).await?;
  let AssetRequestOutput {
    asset,
    discovered_assets,
    dependencies,
  } = result.as_asset().unwrap();

  let mut futures = vec![];
  for dependency in dependencies {
    let future = Box::pin(run_path_request(
      request_context.clone(),
      dependency.clone(),
    ));
    futures.push(tokio::spawn(future));
  }

  for asset in discovered_assets {
    for dependency in &asset.dependencies {
      let future = run_path_request(request_context.clone(), dependency.clone());
      futures.push(tokio::spawn(future));
    }
  }

  for future in futures {
    future.await??;
  }

  Ok(())
}

// #[cfg(test)]
// mod tests {
//   use std::path::{Path, PathBuf};
//   use std::sync::Arc;

//   use atlaspack_core::types::{AtlaspackOptions, Code};
//   use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
//   use atlaspack_filesystem::FileSystem;

//   use crate::requests::{AssetGraphRequest, RequestResult};
//   use crate::test_utils::{request_tracker, RequestTrackerTestOptions};

//   #[tokio::test(flavor = "multi_thread")]
//   async fn test_asset_graph_request_with_no_entries() {
//     let options = RequestTrackerTestOptions::default();
//     let mut request_tracker = request_tracker(options);

//     let asset_graph_request = AssetGraphRequest {};
//     let RequestResult::AssetGraph(asset_graph_request_result) = request_tracker
//       .run_request(asset_graph_request)
//       .await
//       .unwrap()
//     else {
//       assert!(false, "Got invalid result");
//       return;
//     };

//     assert_eq!(asset_graph_request_result.graph.assets.len(), 0);
//     assert_eq!(asset_graph_request_result.graph.dependencies.len(), 0);
//   }

//   #[tokio::test(flavor = "multi_thread")]
//   async fn test_asset_graph_request_with_a_single_entry_with_no_dependencies() {
//     #[cfg(not(target_os = "windows"))]
//     let temporary_dir = PathBuf::from("/atlaspack_tests");
//     #[cfg(target_os = "windows")]
//     let temporary_dir = PathBuf::from("c:/windows/atlaspack_tests");

//     assert!(temporary_dir.is_absolute());

//     let fs = InMemoryFileSystem::default();

//     fs.create_directory(&temporary_dir).unwrap();
//     fs.set_current_working_directory(&temporary_dir); // <- resolver is broken without this
//     fs.write_file(
//       &temporary_dir.join("entry.js"),
//       String::from(
//         r#"
//           console.log('hello world');
//         "#,
//       ),
//     );

//     fs.write_file(&temporary_dir.join("package.json"), String::from("{}"));

//     let mut request_tracker = request_tracker(RequestTrackerTestOptions {
//       atlaspack_options: AtlaspackOptions {
//         entries: vec![temporary_dir.join("entry.js").to_str().unwrap().to_string()],
//         ..AtlaspackOptions::default()
//       },
//       fs: Arc::new(fs),
//       project_root: temporary_dir.clone(),
//       search_path: temporary_dir.clone(),
//       ..RequestTrackerTestOptions::default()
//     });

//     let asset_graph_request = AssetGraphRequest {};
//     let RequestResult::AssetGraph(asset_graph_request_result) = request_tracker
//       .run_request(asset_graph_request)
//       .await
//       .expect("Failed to run asset graph request")
//     else {
//       assert!(false, "Got invalid result");
//       return;
//     };

//     assert_eq!(asset_graph_request_result.graph.assets.len(), 1);
//     assert_eq!(asset_graph_request_result.graph.dependencies.len(), 1);
//     assert_eq!(
//       asset_graph_request_result
//         .graph
//         .assets
//         .get(0)
//         .unwrap()
//         .asset
//         .file_path,
//       temporary_dir.join("entry.js")
//     );
//     assert_eq!(
//       asset_graph_request_result
//         .graph
//         .assets
//         .get(0)
//         .unwrap()
//         .asset
//         .code,
//       (Code::from(
//         String::from(
//           r#"
//             console.log('hello world');
//         "#
//         )
//         .trim_start()
//         .trim_end_matches(|p| p == ' ')
//         .to_string()
//       ))
//     );
//   }

//   #[tokio::test(flavor = "multi_thread")]
//   async fn test_asset_graph_request_with_a_couple_of_entries() {
//     #[cfg(not(target_os = "windows"))]
//     let temporary_dir = PathBuf::from("/atlaspack_tests");
//     #[cfg(target_os = "windows")]
//     let temporary_dir = PathBuf::from("C:\\windows\\atlaspack_tests");

//     let core_path = temporary_dir.join("atlaspack_core");
//     let fs = InMemoryFileSystem::default();

//     fs.create_directory(&temporary_dir).unwrap();
//     fs.set_current_working_directory(&temporary_dir);

//     fs.write_file(
//       &temporary_dir.join("entry.js"),
//       String::from(
//         r#"
//           import {x} from './a';
//           import {y} from './b';
//           console.log(x + y);
//         "#,
//       ),
//     );

//     fs.write_file(
//       &temporary_dir.join("a.js"),
//       String::from(
//         r#"
//           export const x = 15;
//         "#,
//       ),
//     );

//     fs.write_file(
//       &temporary_dir.join("b.js"),
//       String::from(
//         r#"
//           export const y = 27;
//         "#,
//       ),
//     );

//     fs.write_file(&temporary_dir.join("package.json"), String::from("{}"));

//     setup_core_modules(&fs, &core_path);

//     let mut request_tracker = request_tracker(RequestTrackerTestOptions {
//       fs: Arc::new(fs),
//       atlaspack_options: AtlaspackOptions {
//         core_path,
//         entries: vec![temporary_dir.join("entry.js").to_str().unwrap().to_string()],
//         ..AtlaspackOptions::default()
//       },
//       project_root: temporary_dir.clone(),
//       search_path: temporary_dir.clone(),
//       ..RequestTrackerTestOptions::default()
//     });

//     let asset_graph_request = AssetGraphRequest {};
//     let RequestResult::AssetGraph(asset_graph_request_result) = request_tracker
//       .run_request(asset_graph_request)
//       .await
//       .expect("Failed to run asset graph request")
//     else {
//       assert!(false, "Got invalid result");
//       return;
//     };

//     // Entry, 2 assets + helpers file
//     assert_eq!(asset_graph_request_result.graph.assets.len(), 4);
//     // Entry, entry to assets (2), assets to helpers (2)
//     assert_eq!(asset_graph_request_result.graph.dependencies.len(), 5);

//     assert_eq!(
//       asset_graph_request_result
//         .graph
//         .assets
//         .get(0)
//         .unwrap()
//         .asset
//         .file_path,
//       temporary_dir.join("entry.js")
//     );
//   }

//   fn setup_core_modules(fs: &InMemoryFileSystem, core_path: &Path) {
//     let transformer_path = core_path
//       .join("node_modules")
//       .join("@atlaspack/transformer-js");

//     fs.write_file(&transformer_path.join("package.json"), String::from("{}"));
//     fs.write_file(
//       &transformer_path.join("src").join("esmodule-helpers.js"),
//       String::from("/* helpers */"),
//     );
//   }
// }
