use std::sync::Arc;

use petgraph::stable_graph::StableDiGraph;

use crate::{request_tracker::RunRequestError, requests::RequestResult};

pub type RequestGraph = StableDiGraph<RequestNode, RequestEdgeType>;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum RequestNode {
  Error(RunRequestError),
  Root,
  Incomplete(Option<Arc<RequestResult>>),
  Valid(Arc<RequestResult>),
  Invalid(Option<Arc<RequestResult>>),
  FileInvalidation,
}

#[derive(Debug)]
pub enum RequestEdgeType {
  SubRequest,
  FileChangeInvalidation,
}
