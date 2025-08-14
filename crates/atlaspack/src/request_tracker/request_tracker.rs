use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use atlaspack_core::types::Invalidation;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;

use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_filesystem::FileSystemRef;
use petgraph::visit::Dfs;
use petgraph::visit::Reversed;

use crate::plugins::PluginsRef;
use crate::requests::RequestResult;
use crate::AtlaspackError;
use crate::WatchEvent;
use crate::WatchEvents;

use super::Request;
use super::RequestEdgeType;
use super::RequestGraph;
use super::RequestId;
use super::RequestNode;
use super::ResultAndInvalidations;
use super::RunRequestError;
use super::{RunRequestContext, RunRequestMessage};

/// [`RequestTracker`] runs atlaspack work items and constructs a graph of their dependencies.
///
/// Whenever a [`Request`] implementation needs to get the result of another piece of work, it'll
/// make a call into [`RequestTracker`] through its [`RunRequestContext`] abstraction. The request
/// tracker will verify if the piece of work has been completed and return its result. If the work
/// has not been seen yet, it'll be scheduled for execution.
///
/// By asking for the result of a piece of work (through [`RunRequestContext::queue_request`]) a
/// request is creating an edge between itself and that sub-request.
///
/// This will be used to trigger cache invalidations.
pub struct RequestTracker {
  config_loader: ConfigLoaderRef,
  file_system: FileSystemRef,
  graph: RequestGraph,
  options: Arc<AtlaspackOptions>,
  plugins: PluginsRef,
  project_root: PathBuf,
  request_index: HashMap<u64, NodeIndex>,
  invalidations: HashMap<PathBuf, NodeIndex>,
}

impl RequestTracker {
  pub fn new(
    config_loader: ConfigLoaderRef,
    file_system: FileSystemRef,
    options: Arc<AtlaspackOptions>,
    plugins: PluginsRef,
    project_root: PathBuf,
  ) -> Self {
    let mut graph = StableDiGraph::<RequestNode, RequestEdgeType>::new();

    graph.add_node(RequestNode::Root);

    RequestTracker {
      config_loader,
      file_system,
      graph,
      plugins,
      project_root,
      request_index: HashMap::new(),
      invalidations: HashMap::new(),
      options,
    }
  }

  /// Run a request that has no parent. Return the result.
  ///
  /// ## Multi-threading
  /// Sub-requests may be queued from this initial `request` using
  /// [`RunRequestContext::queue_request`].
  /// All sub-requests will run on separate tasks on a thread-pool.
  ///
  /// A request may use the channel passed into  [`RunRequestContext::queue_request`] to wait for
  /// results from sub-requests.
  ///
  /// Because threads will be blocked by waiting on sub-requests, the system may stall if the thread
  /// pool runs out of threads. For the same reason, the number of threads must always be greater
  /// than 1. For this reason the minimum number of threads our thread-pool uses is 4.
  ///
  /// There are multiple ways we can fix this in our implementation:
  /// * Use async, so we get cooperative multi-threading and don't need to worry about this
  /// * Whenever we block a thread, block using recv_timeout and then use [`rayon::yield_now`] so
  ///   other tasks get a chance to tick on our thread-pool. This is a very poor implementation of
  ///   the cooperative threading behaviours async will grant us.
  /// * Don't use rayon for multi-threading here and use a custom thread-pool implementation which
  ///   ensures we always have more threads than concurrently running requests
  /// * Run requests that need to spawn multithreaded sub-requests on the main-thread
  ///   - That is, introduce a new `MainThreadRequest` trait, which is able to enqueue requests,
  ///     these will run on the main-thread, therefore it'll be simpler to implement queueing
  ///     without stalls and locks/channels
  ///   - For non-main-thread requests, do not allow enqueueing of sub-requests
  pub async fn run_request(&mut self, request: impl Request) -> anyhow::Result<RequestResult> {
    tracing::debug!("run_request {:?}", request);

    let request_id = request.id();
    let (tx, rx) = std::sync::mpsc::channel();
    let tx2 = tx.clone();
    let _ = tx.send(RequestQueueMessage::RunRequest {
      tx: tx2,
      message: RunRequestMessage {
        request: Box::new(request),
        parent_request_id: None,
        response_tx: None,
      },
    });
    drop(tx);

    let mut tasks = Vec::new();

    while let Ok(request_queue_message) = rx.recv() {
      match request_queue_message {
        RequestQueueMessage::RunRequest {
          message:
            RunRequestMessage {
              request,
              parent_request_id,
              response_tx,
            },
          tx,
        } => {
          let request_id = request.id();
          tracing::trace!(?request_id, ?parent_request_id, "Run request");

          if self.prepare_request(request_id)? {
            let context = RunRequestContext::new(
              self.config_loader.clone(),
              self.file_system.clone(),
              self.options.clone(),
              Some(request_id),
              self.plugins.clone(),
              self.project_root.clone(),
              // sub-request run
              Box::new({
                let tx = tx.clone();
                move |message| {
                  let tx2 = tx.clone();
                  tx.send(RequestQueueMessage::RunRequest { message, tx: tx2 })
                    .unwrap();
                }
              }),
            );

            let task = tokio::spawn({
              let tx = tx.clone();
              async move {
                let result = request.run(context).await;
                let _ = tx.send(RequestQueueMessage::RequestResult {
                  request_id,
                  parent_request_id,
                  result,
                  response_tx,
                });
              }
            });

            tasks.push(task);
          } else {
            // Cached request
            if let Some(response_tx) = response_tx {
              let result = self.get_request(parent_request_id, request_id);
              let _ = response_tx.send(result.map(|r| (r, request_id)));
            }
          };
        }
        RequestQueueMessage::RequestResult {
          request_id,
          parent_request_id,
          result,
          response_tx,
        } => {
          tracing::trace!(?request_id, ?parent_request_id, "Request result");
          self.store_request(request_id, &result)?;
          self.link_request_to_parent(request_id, parent_request_id)?;

          if let Some(response_tx) = response_tx {
            let _ = response_tx.send(result.map(|result| (result.result, request_id)));
          }
        }
      }
    }

    for task in tasks {
      task.await?;
    }

    self.get_request(None, request_id)
  }

  /// Before a request is run, a 'pending' [`RequestNode::Incomplete`] entry is added to the graph.
  fn prepare_request(&mut self, request_id: u64) -> anyhow::Result<bool> {
    let node_index = self
      .request_index
      .entry(request_id)
      .or_insert_with(|| self.graph.add_node(RequestNode::Incomplete));

    let request_node = self
      .graph
      .node_weight_mut(*node_index)
      .ok_or_else(|| diagnostic_error!("Failed to find request node"))?;

    // Don't run if already run
    if let RequestNode::Valid(_) = request_node {
      return Ok(false);
    }

    *request_node = RequestNode::Incomplete;
    Ok(true)
  }

  /// Once a request finishes, its result is stored under its [`RequestNode`] entry on the graph
  fn store_request(
    &mut self,
    request_id: u64,
    result: &Result<ResultAndInvalidations, RunRequestError>,
  ) -> anyhow::Result<()> {
    let node_index = self
      .request_index
      .get(&request_id)
      .ok_or_else(|| diagnostic_error!("Failed to find request"))?;

    let request_node = self
      .graph
      .node_weight_mut(*node_index)
      .ok_or_else(|| diagnostic_error!("Failed to find request"))?;

    // Update node with latest result
    *request_node = match result {
      Ok(result) => RequestNode::Valid(result.result.clone()),
      Err(error) => RequestNode::Error(AtlaspackError::from(error).into()),
    };

    // Assign invalidations
    if let Ok(result) = result {
      for invalidation in result.invalidations.iter() {
        match invalidation {
          Invalidation::FileChange(file_path) => {
            let invalidation_node = self
              .invalidations
              .entry(file_path.clone())
              .or_insert_with(|| self.graph.add_node(RequestNode::FileInvalidation));

            tracing::trace!(
              "Add {:?} as invalidation for {:?}",
              file_path
                .strip_prefix(&self.project_root)
                .unwrap_or(file_path),
              self.graph[*node_index],
            );
            self.graph.add_edge(
              *node_index,
              *invalidation_node,
              RequestEdgeType::FileChangeInvalidation,
            );
          }
        }
      }
    }

    Ok(())
  }

  /// Get a request result and call [`RequestTracker::link_request_to_parent`] to create a
  /// dependency link between the source request and this sub-request.
  fn get_request(
    &mut self,
    parent_request_hash: Option<u64>,
    request_id: u64,
  ) -> anyhow::Result<RequestResult> {
    self.link_request_to_parent(request_id, parent_request_hash)?;

    let Some(node_index) = self.request_index.get(&request_id) else {
      return Err(diagnostic_error!("Impossible error"));
    };
    let Some(request_node) = self.graph.node_weight(*node_index) else {
      return Err(diagnostic_error!("Impossible"));
    };

    match request_node {
      RequestNode::Error(error) => Err(AtlaspackError::from(error).into()),
      RequestNode::Valid(value) => Ok(value.clone()),
      _ => Err(diagnostic_error!("Impossible")),
    }
  }

  /// Create an edge between a parent request and the target request.
  fn link_request_to_parent(
    &mut self,
    request_id: u64,
    parent_request_hash: Option<u64>,
  ) -> anyhow::Result<()> {
    let Some(node_index) = self.request_index.get(&request_id) else {
      return Err(diagnostic_error!("Impossible error"));
    };

    if let Some(parent_request_id) = parent_request_hash {
      let parent_node_index = self
        .request_index
        .get(&parent_request_id)
        .ok_or_else(|| diagnostic_error!("Failed to find requests"))?;

      self
        .graph
        .add_edge(*parent_node_index, *node_index, RequestEdgeType::SubRequest);
    } else {
      self
        .graph
        .add_edge(NodeIndex::new(0), *node_index, RequestEdgeType::SubRequest);
    }
    Ok(())
  }

  #[tracing::instrument(level = "info", skip_all)]
  pub fn respond_to_fs_events(&mut self, watch_events: WatchEvents) -> bool {
    tracing::info!("Responding to {} watch events", watch_events.len());
    let mut need_rebuild = false;

    for invalidation in watch_events.iter() {
      // We don't currently distinguish between the different file event types
      match invalidation {
        WatchEvent::Delete(file_path)
        | WatchEvent::Update(file_path)
        | WatchEvent::Create(file_path) => {
          if let Some(invalidation_node) = self.invalidations.get(file_path) {
            let mut invalid_nodes = Vec::new();
            {
              let reverse_graph = Reversed(&self.graph);
              let mut dfs = Dfs::new(reverse_graph, *invalidation_node);

              while let Some(node_index) = dfs.next(reverse_graph) {
                invalid_nodes.push(node_index);
              }
            }

            for invalid_node in invalid_nodes {
              tracing::info!(
                "{:?} invalidates {:#?}",
                file_path,
                self.graph.node_weight(invalid_node)
              );
              self.graph[invalid_node] = RequestNode::Invalid;
              need_rebuild = true;
            }
          }
        }
      }
    }

    need_rebuild
  }
}

/// Internally, [`RequestTracker`] ticks a queue of work related to each 'entry request' ran.
///
/// This enum represents messages that can be sent to the main-thread that is ticking the work for
/// an entry from worker threads that are processing individual requests.
///
/// See [`RequestTracker::run_request`].
#[derive(Debug)]
enum RequestQueueMessage {
  RunRequest {
    tx: Sender<RequestQueueMessage>,
    message: RunRequestMessage,
  },
  RequestResult {
    request_id: RequestId,
    parent_request_id: Option<RequestId>,
    result: Result<ResultAndInvalidations, RunRequestError>,
    response_tx: Option<Sender<anyhow::Result<(RequestResult, RequestId)>>>,
  },
}
