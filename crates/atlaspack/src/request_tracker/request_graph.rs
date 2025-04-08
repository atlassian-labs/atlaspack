use std::sync::Arc;

use petgraph::stable_graph::StableDiGraph;

use crate::{request_tracker::RunRequestError, requests::RequestResult};

pub type RequestGraph = StableDiGraph<RequestNode, RequestEdgeType>;

#[derive(Debug)]
pub enum RequestNode {
  Error(RunRequestError),
  Root,
  Incomplete,
  Valid(Arc<RequestResult>),
  Invalid(Arc<RequestResult>),
  FileInvalidation,
}

#[derive(Debug)]
pub enum RequestEdgeType {
  SubRequest,
  FileChangeInvalidation,
}
