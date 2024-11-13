use std::any::Any;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::Invalidation;
use atlaspack_filesystem::FileSystemRef;
use dyn_hash::DynHash;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use crate::plugins::PluginsRef;
use crate::request_tracker::BroadcastRequestError;
use crate::requests::RequestResult;

use super::RequestQueueMessage;
use super::RequestTrackerState;

#[derive(Debug)]
pub struct RunRequestMessage {
  pub request: Box<dyn Request>,
  pub parent_request_id: Option<u64>,
  pub response_tx: Option<Sender<Result<(RequestResult, RequestId), anyhow::Error>>>,
}

/// This is the API for requests to call back onto the `RequestTracker`.
///
/// We want to avoid exposing internals of the request tracker to the implementations so that we
/// can change this.
pub struct RunRequestContext {
  request_id: u64,
  config_loader: ConfigLoaderRef,
  file_system: FileSystemRef,
  pub options: Arc<AtlaspackOptions>,
  parent_request_id: Option<u64>,
  plugins: PluginsRef,
  pub project_root: PathBuf,
  state: Arc<RwLock<RequestTrackerState>>,
}

impl RunRequestContext {
  pub(crate) fn new(
    request_id: u64,
    config_loader: ConfigLoaderRef,
    file_system: FileSystemRef,
    options: Arc<AtlaspackOptions>,
    parent_request_id: Option<u64>,
    plugins: PluginsRef,
    project_root: PathBuf,
    state: Arc<RwLock<RequestTrackerState>>,
  ) -> Self {
    Self {
      request_id,
      config_loader,
      file_system,
      options,
      parent_request_id,
      plugins,
      project_root,
      state,
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
  pub async fn run_request<R: Request + Any>(
    &self,
    request: R,
  ) -> anyhow::Result<Arc<RequestResult>> {
    let parent_request_id = self.request_id;
    let request_id = request.id();

    let mut state = self.state.write().await;
    let result = state.get_pending_request(Some(parent_request_id), request_id);

    if let Some(pending_future) = result {
      // Release the lock before waiting for the request to finish
      drop(state);
      let result = pending_future.await?;

      return Ok(result);
    } else {
      let context = RunRequestContext {
        request_id,
        config_loader: self.config_loader.clone(),
        file_system: self.file_system.clone(),
        options: self.options.clone(),
        parent_request_id: Some(parent_request_id),
        plugins: self.plugins.clone(),
        project_root: self.project_root.clone(),
        state: self.state.clone(),
      };

      // tracing::info!(
      //   "Running request {}::{}",
      //   std::any::type_name::<R>(),
      //   request_id
      // );

      let tx = state.register_pending_request(request_id);

      // Drop the lock before running the request
      drop(state);
      let request_result = request.run(context).await;

      // Now we will lock state again and put the result in
      let output_result = request_result
        .as_ref()
        .map(|result| result.result().clone())
        .map_err(|_| anyhow!("TODO"));
      // Order matters; first we store the result, then we broadcast.
      let mut state = self.state.write().await;
      let broadcast_result = request_result
        .as_ref()
        .map(|result| result.result().clone())
        .map_err(|_| BroadcastRequestError::Any);
      state.store_request(request_id, request_result)?;

      tracing::trace!(
        "Broadcasting request result {}::{:?}",
        std::any::type_name::<R>(),
        request
      );
      // Ignore errors here, because we don't care if there are no receivers.
      // There will only be receivers if something is waiting on the result.
      let _ = tx.send(broadcast_result);
      drop(state);

      output_result
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
    self.type_id().hash(&mut hasher);
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
  result: Arc<RequestResult>,
  invalidations: Vec<Invalidation>,
}

impl ResultAndInvalidations {
  pub fn new(result: RequestResult, invalidations: Vec<Invalidation>) -> Self {
    Self {
      result: Arc::new(result),
      invalidations,
    }
  }

  pub fn result(&self) -> &Arc<RequestResult> {
    &self.result
  }

  pub fn invalidations(&self) -> &[Invalidation] {
    &self.invalidations
  }
}
