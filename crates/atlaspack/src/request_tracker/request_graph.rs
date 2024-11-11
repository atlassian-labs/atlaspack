use std::sync::Arc;

use petgraph::stable_graph::StableDiGraph;

use crate::{
  request_tracker::{ResultAndInvalidations, RunRequestError},
  requests::RequestResult,
};

pub type RequestGraph = StableDiGraph<RequestNode, RequestEdgeType>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum BroadcastRequestError {
  #[error("Failure running request")]
  Any,
}

#[derive(Debug)]
pub enum RequestNode {
  Root,
  Incomplete(tokio::sync::broadcast::Receiver<Result<Arc<RequestResult>, BroadcastRequestError>>),
  Valid(ResultAndInvalidations),
  Error(RunRequestError),
}

#[derive(Debug)]
pub enum RequestEdgeType {
  SubRequest,
}
