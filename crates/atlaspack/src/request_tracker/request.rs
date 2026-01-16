use atlaspack_memoization_cache::CacheHandler;
use atlaspack_memoization_cache::CacheHandlerTrait;
use atlaspack_memoization_cache::InMemoryReaderWriter;
use atlaspack_memoization_cache::LmdbCacheReaderWriter;
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
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::Invalidation;
use atlaspack_filesystem::FileSystemRef;
use dyn_hash::DynHash;

use crate::plugins::PluginsRef;
use crate::requests::RequestResult;

// Allow the enum size differential as the smaller variant (InMemory) is only used
// in tests. We also only have a single instance of this enum at a time
#[allow(clippy::large_enum_variant)]
pub enum DynCacheHandler {
  Lmdb(CacheHandler<LmdbCacheReaderWriter>),
  InMemory(CacheHandler<InMemoryReaderWriter>),
}

impl DynCacheHandler {
  pub fn complete_session(&self) -> anyhow::Result<atlaspack_memoization_cache::StatsSnapshot> {
    match self {
      DynCacheHandler::Lmdb(cache) => cache.complete_session(),
      DynCacheHandler::InMemory(cache) => cache.complete_session(),
    }
  }

  pub async fn run<Input, RunFn, Res, FutureResult, Error>(
    &self,
    input: Input,
    run_fn: RunFn,
  ) -> Result<Res, Error>
  where
    Input: atlaspack_memoization_cache::Cacheable + Send,
    Res: atlaspack_memoization_cache::CacheResponse + Send,
    Error: Send,
    FutureResult: Future<Output = Result<Res, Error>> + Send,
    RunFn: FnOnce(Input) -> FutureResult + Send,
  {
    match self {
      DynCacheHandler::Lmdb(cache) => cache.run(input, run_fn).await,
      DynCacheHandler::InMemory(cache) => cache.run(input, run_fn).await,
    }
  }
}

pub type CacheRef = Arc<DynCacheHandler>;

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
  pub cache: CacheRef,
  pub options: Arc<AtlaspackOptions>,
  parent_request_id: Option<u64>,
  plugins: PluginsRef,
  pub project_root: PathBuf,
  run_request_fn: RunRequestFn,
}

impl RunRequestContext {
  /// Create a test context with an in-memory cache
  #[cfg(test)]
  pub fn new_for_testing(plugins: PluginsRef) -> Self {
    use atlaspack_core::config_loader::ConfigLoader;
    use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
    use atlaspack_memoization_cache::CacheMode;

    let fs = Arc::new(InMemoryFileSystem::default());
    let config_loader = Arc::new(ConfigLoader {
      fs: fs.clone(),
      project_root: PathBuf::default(),
      search_path: PathBuf::default(),
    });

    Self {
      cache: Arc::new(DynCacheHandler::InMemory(CacheHandler::new(
        InMemoryReaderWriter::default(),
        CacheMode::Off,
      ))),
      config_loader,
      file_system: fs,
      options: Arc::new(AtlaspackOptions::default()),
      parent_request_id: None,
      plugins,
      project_root: PathBuf::default(),
      run_request_fn: Box::new(|_| {}),
    }
  }
}

impl RunRequestContext {
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    config_loader: ConfigLoaderRef,
    file_system: FileSystemRef,
    options: Arc<AtlaspackOptions>,
    parent_request_id: Option<u64>,
    plugins: PluginsRef,
    project_root: PathBuf,
    cache: CacheRef,
    run_request_fn: RunRequestFn,
  ) -> Self {
    Self {
      cache,
      config_loader,
      file_system,
      options,
      parent_request_id,
      plugins,
      project_root,
      run_request_fn,
    }
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
        if let Ok(Ok(msg)) = tokio::task::spawn_blocking(move || sync_rx.recv()).await {
          let _ = tx.send(msg);
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
