use std::collections::HashMap;
use std::future::Future;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use futures::future::BoxFuture;
use futures::FutureExt;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;

use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_filesystem::FileSystemRef;

use crate::plugins::PluginsRef;
use crate::requests::RequestResult;

use super::BroadcastRequestError;
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
  options: Arc<AtlaspackOptions>,
  plugins: PluginsRef,
  project_root: PathBuf,
  state: Arc<tokio::sync::RwLock<RequestTrackerState>>,
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

    let state = RequestTrackerState {
      graph,
      request_index: HashMap::new(),
      request_stats: RequestStats::default(),
    };
    let state = Arc::new(tokio::sync::RwLock::new(state));

    RequestTracker {
      config_loader,
      file_system,
      plugins,
      project_root,
      state,
      options,
    }
  }

  pub fn state(&self) -> &Arc<tokio::sync::RwLock<RequestTrackerState>> {
    &self.state
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
    let mut state = self.state.write().await;
    let tx = state.register_pending_request(request_id);
    drop(state);

    let context = RunRequestContext::new(
      request.id(),
      self.config_loader.clone(),
      self.file_system.clone(),
      self.options.clone(),
      Some(request_id),
      self.plugins.clone(),
      self.project_root.clone(),
      self.state.clone(),
    );

    let result = request.run(context).await;
    tx.send(
      result
        .as_ref()
        .map(|r| r.result().clone())
        .map_err(|_| BroadcastRequestError::Any),
    )?;

    result.map(|r| r.result().clone())
  }
}

/// Internally, [`RequestTracker`] ticks a queue of work related to each 'entry request' ran.
///
/// This enum represents messages that can be sent to the main-thread that is ticking the work for
/// an entry from worker threads that are processing individual requests.
///
/// See [`RequestTracker::run_request`].
#[derive(Debug)]
pub enum RequestQueueMessage {
  RunRequest {
    tx: tokio::sync::mpsc::Sender<RequestQueueMessage>,
    message: RunRequestMessage,
  },
  RequestResult {
    request_id: RequestId,
    parent_request_id: Option<RequestId>,
    result: Result<ResultAndInvalidations, RunRequestError>,
    response_tx: Option<tokio::sync::mpsc::Sender<anyhow::Result<(RequestResult, RequestId)>>>,
  },
}

pub struct RequestTrackerState {
  graph: RequestGraph,
  request_index: HashMap<u64, NodeIndex>,
  request_stats: RequestStats,
}

#[derive(Debug, Default)]
pub struct RequestStats {
  total_requests: usize,
  pending_requests: usize,

  asset_requests: usize,
  path_requests: usize,
}

impl RequestTrackerState {
  pub fn get_request_stats(&self) -> &RequestStats {
    &self.request_stats
  }

  pub fn get_pending_request(
    &mut self,
    parent_request_id: Option<u64>,
    request_id: u64,
  ) -> Option<BoxFuture<'static, Result<Arc<RequestResult>, RunRequestError>>> {
    let node_index = *self.request_index.get(&request_id)?;

    self
      .link_request_to_parent(request_id, parent_request_id)
      .unwrap();

    let request_node = self.graph.node_weight(node_index)?;

    match request_node {
      RequestNode::Root => None,
      RequestNode::Error(error) => {
        let error = anyhow!(error.to_string());
        Some(async move { Err(error) }.boxed())
      }
      RequestNode::Valid(result_and_invalidations) => {
        let result = result_and_invalidations.result().clone();
        Some(async move { Ok(result) }.boxed())
      }
      RequestNode::Incomplete(receiver) => {
        let mut receiver = receiver.resubscribe();
        Some(
          async move {
            // TODO: timeout doesn't need to be here ; but it'd be nice to warm if we stall
            // let result =
            //   tokio::time::timeout(Duration::from_millis(3000), receiver.recv()).await???;
            let result = receiver.recv().await??;
            Ok(result)
          }
          .boxed(),
        )
      }
    }
  }

  pub fn register_pending_request(
    &mut self,
    request_id: u64,
  ) -> tokio::sync::broadcast::Sender<Result<Arc<RequestResult>, BroadcastRequestError>> {
    self.request_stats.total_requests += 1;
    self.request_stats.pending_requests += 1;

    let (tx, rx) = tokio::sync::broadcast::channel(1);
    let node = self.graph.add_node(RequestNode::Incomplete(rx));
    self.request_index.insert(request_id, node);
    tx
  }

  /// Before a request is run, a 'pending' [`RequestNode::Incomplete`] entry is added to the graph.
  // pub fn prepare_request(&mut self, request_id: u64) -> anyhow::Result<bool> {
  //   let node_index = self
  //     .request_index
  //     .entry(request_id)
  //     .or_insert_with(|| self.graph.add_node(RequestNode::Incomplete));
  //   let request_node = self
  //     .graph
  //     .node_weight_mut(*node_index)
  //     .ok_or_else(|| diagnostic_error!("Failed to find request node"))?;
  //   // Don't run if already run
  //   if let RequestNode::Valid(_) = request_node {
  //     return Ok(false);
  //   }
  //   *request_node = RequestNode::Incomplete;
  //   Ok(true)
  // }

  /// Once a request finishes, its result is stored under its [`RequestNode`] entry on the graph
  pub fn store_request(
    &mut self,
    request_id: u64,
    result: Result<ResultAndInvalidations, RunRequestError>,
  ) -> anyhow::Result<()> {
    self.request_stats.pending_requests -= 1;

    match &result {
      Ok(r) => match r.result().deref() {
        RequestResult::Asset(_) => self.request_stats.asset_requests += 1,
        RequestResult::Path(_) => self.request_stats.path_requests += 1,
        _ => {}
      },
      _ => {}
    }

    let node_index = self
      .request_index
      .get(&request_id)
      .ok_or_else(|| diagnostic_error!("Failed to find request"))?;

    let request_node = self
      .graph
      .node_weight_mut(*node_index)
      .ok_or_else(|| diagnostic_error!("Failed to find request"))?;

    if let RequestNode::Valid(_) = request_node {
      return Ok(());
    }

    *request_node = match result {
      Ok(result) => RequestNode::Valid(result),
      Err(error) => RequestNode::Error(anyhow!(error.to_string())),
    };

    Ok(())
  }

  /// Get a request result and call [`RequestTracker::link_request_to_parent`] to create a
  /// dependency link between the source request and this sub-request.
  pub fn get_request(
    &mut self,
    parent_request_hash: Option<u64>,
    request_id: u64,
  ) -> anyhow::Result<Arc<RequestResult>> {
    self.link_request_to_parent(request_id, parent_request_hash)?;

    let Some(node_index) = self.request_index.get(&request_id) else {
      return Err(diagnostic_error!("Impossible error"));
    };
    let Some(request_node) = self.graph.node_weight(*node_index) else {
      return Err(diagnostic_error!("Impossible"));
    };

    tracing::debug!("Request node: {:?}", request_node);
    match request_node {
      RequestNode::Error(error) => Err(anyhow!(error.to_string())),
      RequestNode::Valid(value) => Ok(value.result().clone()),
      _ => Err(diagnostic_error!("Invalid request state")),
    }
  }

  /// Create an edge between a parent request and the target request.
  pub fn link_request_to_parent(
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
}
