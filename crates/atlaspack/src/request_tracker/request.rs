use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::task::{Context, Poll};
use tokio::sync::oneshot;

use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::Invalidation;
use atlaspack_filesystem::FileSystemRef;
use dyn_hash::DynHash;

use crate::plugins::PluginsRef;
use crate::requests::RequestResult;

type ChannelRequestResult = anyhow::Result<(Arc<RequestResult>, RequestId, bool)>;
pub type RequestResultReceiver = Receiver<ChannelRequestResult>;
pub type RequestResultSender = Sender<ChannelRequestResult>;
type AsyncRequestResult = anyhow::Result<(Arc<RequestResult>, RequestId, bool)>;

#[derive(Debug)]
pub struct RunRequestMessage {
  pub request: Box<dyn Request>,
  pub parent_request_id: Option<u64>,
  pub response_tx: Option<RequestResultSender>,
}

type RunRequestFn = Box<dyn Fn(RunRequestMessage) + Send + Sync>;

/// A future that represents an executing request
pub struct ExecuteRequestFuture {
  queue_result: anyhow::Result<()>,
  receiver: oneshot::Receiver<AsyncRequestResult>,
  _bridge_task: tokio::task::JoinHandle<()>,
}

impl Future for ExecuteRequestFuture {
  type Output = anyhow::Result<(Arc<RequestResult>, RequestId, bool)>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    // First check if queueing failed
    if let Err(e) = &self.queue_result {
      return Poll::Ready(Err(anyhow::anyhow!("Failed to queue request: {}", e)));
    }

    // Poll the oneshot receiver
    match Pin::new(&mut self.receiver).poll(cx) {
      Poll::Ready(Ok(result)) => Poll::Ready(result),
      Poll::Ready(Err(_)) => Poll::Ready(Err(anyhow::anyhow!("Request was cancelled or failed"))),
      Poll::Pending => Poll::Pending,
    }
  }
}

/// This is the API for requests to call back onto the `RequestTracker`.
///
/// We want to avoid exposing internals of the request tracker to the implementations so that we
/// can change this.
pub struct RunRequestContext {
  config_loader: ConfigLoaderRef,
  file_system: FileSystemRef,
  pub options: Arc<AtlaspackOptions>,
  parent_request_id: Option<u64>,
  plugins: PluginsRef,
  pub project_root: PathBuf,
  run_request_fn: RunRequestFn,
}

impl RunRequestContext {
  pub(crate) fn new(
    config_loader: ConfigLoaderRef,
    file_system: FileSystemRef,
    options: Arc<AtlaspackOptions>,
    parent_request_id: Option<u64>,
    plugins: PluginsRef,
    project_root: PathBuf,
    run_request_fn: RunRequestFn,
  ) -> Self {
    Self {
      config_loader,
      file_system,
      options,
      parent_request_id,
      plugins,
      project_root,
      run_request_fn,
    }
  }

  /// Report an event
  pub async fn report(&self, event: ReporterEvent) {
    self
      .plugins()
      .reporter()
      .report(&event)
      .await
      .expect("TODO this should be handled?")
  }

  /// Run a child request to the current request
  pub fn queue_request(
    &mut self,
    request: impl Request,
    tx: RequestResultSender,
  ) -> anyhow::Result<()> {
    let request: Box<dyn Request> = Box::new(request);
    let message = RunRequestMessage {
      request,
      response_tx: Some(tx),
      parent_request_id: self.parent_request_id,
    };
    (*self.run_request_fn)(message);
    Ok(())
  }

  /// Execute a child request and return a future that resolves to its result.
  /// This is an async version of queue_request that doesn't require managing channels manually.
  pub fn execute_request(&mut self, request: impl Request) -> ExecuteRequestFuture {
    let (tx, rx) = oneshot::channel();

    // Create a wrapper sender that forwards to the oneshot channel
    let (sync_tx, sync_rx) = std::sync::mpsc::channel();
    let queue_result = self.queue_request(request, sync_tx);

    ExecuteRequestFuture {
      queue_result,
      receiver: rx,
      _bridge_task: tokio::spawn(async move {
        if let Ok(result) = tokio::task::spawn_blocking(move || sync_rx.recv()).await {
          if let Ok(msg) = result {
            let _ = tx.send(msg);
          }
        }
      }),
    }
  }

  pub fn file_system(&self) -> &FileSystemRef {
    &self.file_system
  }

  pub fn plugins(&self) -> &PluginsRef {
    &self.plugins
  }

  pub fn config(&self) -> &ConfigLoaderRef {
    &self.config_loader
  }
}

// We can type this properly
pub type RunRequestError = anyhow::Error;
pub type RequestId = u64;

#[async_trait]
pub trait Request: DynHash + Send + Sync + Debug + 'static {
  fn id(&self) -> RequestId {
    let mut hasher = atlaspack_core::hash::IdentifierHasher::default();
    std::any::type_name::<Self>().hash(&mut hasher);
    self.dyn_hash(&mut hasher);
    hasher.finish()
  }

  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError>;
}

dyn_hash::hash_trait_object!(Request);

#[derive(Debug, Clone, PartialEq)]
pub struct ResultAndInvalidations {
  pub result: RequestResult,
  pub invalidations: Vec<Invalidation>,
}
