use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use atlaspack_core::types::Invalidation;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;

use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_filesystem::FileSystemRef;
use petgraph::Direction;
use petgraph::visit::Dfs;
use petgraph::visit::EdgeRef;
use petgraph::visit::Reversed;

use crate::AtlaspackError;
use crate::WatchEvent;
use crate::WatchEvents;
use crate::plugins::PluginsRef;
use crate::request_tracker::RequestResultSender;
use crate::requests::RequestResult;

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
  invalid_nodes: HashSet<NodeIndex>,
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
      invalid_nodes: HashSet::new(),
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
  pub async fn run_request(&mut self, request: impl Request) -> anyhow::Result<Arc<RequestResult>> {
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

          if let Some(previous_result) = self.prepare_request(request_id)? {
            self.link_request_to_parent(request_id, parent_request_id)?;

            // Cached request
            if let Some(response_tx) = response_tx {
              let _ = response_tx.send(Ok((previous_result, request_id, true)));
            }
            continue;
          }

          // Request needs to be executed
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

          tokio::spawn({
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
        }
        RequestQueueMessage::RequestResult {
          request_id,
          parent_request_id,
          result,
          response_tx,
        } => {
          tracing::trace!(?request_id, ?parent_request_id, "Request result");
          let result = self.store_request(request_id, result);
          self.link_request_to_parent(request_id, parent_request_id)?;

          if let Some(response_tx) = response_tx {
            let _ = response_tx.send(result.map(|result| (result, request_id, false)));
          }
        }
      }
    }

    self.link_request_to_parent(request_id, None)?;
    self.get_request(request_id)
  }

  pub fn get_invalid_nodes(&self) -> impl Iterator<Item = &RequestNode> {
    self
      .invalid_nodes
      .iter()
      .map(|node_index| &self.graph[*node_index])
  }

  /// Before a request is run, a 'pending' [`RequestNode::Incomplete`] entry is added to the graph.
  fn prepare_request(&mut self, request_id: u64) -> anyhow::Result<Option<Arc<RequestResult>>> {
    let node_index = *self
      .request_index
      .entry(request_id)
      .or_insert_with(|| self.graph.add_node(RequestNode::Incomplete(None)));

    let request_node = self
      .graph
      .node_weight_mut(node_index)
      .ok_or_else(|| diagnostic_error!("Failed to find request node"))?;

    // Don't run if already run
    if let RequestNode::Valid(previous_result) = request_node {
      return Ok(Some(previous_result.clone()));
    }

    self.invalid_nodes.remove(&node_index);
    *request_node = if let RequestNode::Invalid(previous_result) = request_node {
      RequestNode::Incomplete(previous_result.clone())
    } else {
      RequestNode::Incomplete(None)
    };

    self.clear_invalidations(node_index);

    Ok(None)
  }

  /// Cleans up old invalidations before a request is executed
  fn clear_invalidations(&mut self, node_index: NodeIndex) {
    let mut old_invalidations = Vec::new();
    for edge in self.graph.edges_directed(node_index, Direction::Incoming) {
      if let RequestEdgeType::FileChangeInvalidation = edge.weight() {
        old_invalidations.push(edge.id());
      }
    }
    for edge_id in old_invalidations {
      self.graph.remove_edge(edge_id);
    }
  }

  /// Once a request finishes, its result is stored under its [`RequestNode`] entry on the graph
  fn store_request(
    &mut self,
    request_id: u64,
    result: Result<ResultAndInvalidations, RunRequestError>,
  ) -> anyhow::Result<Arc<RequestResult>> {
    let node_index = self
      .request_index
      .get(&request_id)
      .ok_or_else(|| diagnostic_error!("Failed to find request"))?;

    let request_node = self
      .graph
      .node_weight_mut(*node_index)
      .ok_or_else(|| diagnostic_error!("Failed to find request"))?;

    match result {
      Err(error) => {
        self.invalid_nodes.insert(*node_index);
        *request_node = RequestNode::Error(AtlaspackError::from(&error).into());

        Err(error)
      }
      Ok(result) => {
        let ResultAndInvalidations {
          result,
          invalidations,
        } = result;
        let result = Arc::new(result);

        // Update node with latest result
        *request_node = RequestNode::Valid(result.clone());

        for invalidation in invalidations.iter() {
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

        Ok(result)
      }
    }
  }

  fn get_request_node(&self, request_id: u64) -> Option<&RequestNode> {
    let node_index = self.request_index.get(&request_id)?;
    self.graph.node_weight(*node_index)
  }

  fn get_request(&self, request_id: u64) -> anyhow::Result<Arc<RequestResult>> {
    match self.get_request_node(request_id) {
      Some(RequestNode::Error(error)) => Err(AtlaspackError::from(error).into()),
      Some(RequestNode::Valid(value)) => Ok(value.clone()),
      _ => Err(diagnostic_error!("No request with result {}", request_id)),
    }
  }

  pub fn get_cached_request_result(&self, request: impl Request) -> Option<Arc<RequestResult>> {
    match self.get_request_node(request.id())? {
      RequestNode::Valid(value) => Some(value.clone()),
      RequestNode::Invalid(value) => value.as_ref().map(|v| v.clone()),
      _ => None,
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

  fn invalidate_node(&mut self, node_index: &NodeIndex, file_path_reason: &PathBuf) {
    let mut invalid_nodes = Vec::new();
    {
      let reverse_graph = Reversed(&self.graph);
      let mut dfs = Dfs::new(reverse_graph, *node_index);

      while let Some(node_index) = dfs.next(reverse_graph) {
        let node = &self.graph[node_index];

        match node {
          RequestNode::Incomplete(_) | RequestNode::Valid(_) => {
            invalid_nodes.push(node_index);
          }
          // Ignore the following node types
          RequestNode::Root => {}
          RequestNode::FileInvalidation => {}
          RequestNode::Error(_) => {}
          RequestNode::Invalid(_) => {}
        }
      }
    }

    for invalid_node in invalid_nodes {
      self.graph[invalid_node] = match &self.graph[invalid_node] {
        RequestNode::Valid(result) => {
          tracing::info!("{:?} invalidates {}", file_path_reason, result);

          RequestNode::Invalid(Some(result.clone()))
        }
        RequestNode::Incomplete(result) => RequestNode::Invalid(result.clone()),
        _ => {
          panic!("Impossible node type")
        }
      };
      self.invalid_nodes.insert(invalid_node);
    }
  }

  #[tracing::instrument(level = "debug", skip_all, ret, fields(events = watch_events.len()))]
  pub fn respond_to_fs_events(&mut self, watch_events: WatchEvents) -> bool {
    let nodes_to_invalidate: Vec<(NodeIndex, &PathBuf)> = watch_events
      .iter()
      .filter_map(|invalidation| {
        // We don't currently distinguish between the different file event types
        match invalidation {
          WatchEvent::Delete(file_path)
          | WatchEvent::Update(file_path)
          | WatchEvent::Create(file_path) => {
            self.invalidations.get(file_path).map(|n| (*n, file_path))
          }
        }
      })
      .collect();

    for (node_id, file_path_reason) in nodes_to_invalidate.iter() {
      self.invalidate_node(node_id, file_path_reason);
    }

    tracing::debug!(
      "Invalid nodes {:#?}",
      self
        .invalid_nodes
        .iter()
        .map(|node_id| &self.graph[*node_id])
    );

    // We need to rebuild if there were any invalidated nodes from the file
    // events
    !nodes_to_invalidate.is_empty() ||
    // or if there are still any remaining invalid nodes (e.g. Failed requests)
    !self.invalid_nodes.is_empty()
  }
}

/// Internally, [`RequestTracker`] ticks a queue of work related to each 'entry request' ran.
///
/// This enum represents messages that can be sent to the main-thread that is ticking the work for
/// an entry from worker threads that are processing individual requests.
///
/// See [`RequestTracker::run_request`].
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum RequestQueueMessage {
  RunRequest {
    tx: Sender<RequestQueueMessage>,
    message: RunRequestMessage,
  },
  RequestResult {
    request_id: RequestId,
    parent_request_id: Option<RequestId>,
    result: Result<ResultAndInvalidations, RunRequestError>,
    response_tx: Option<RequestResultSender>,
  },
}
