use petgraph::stable_graph::StableDiGraph;

use crate::{request_tracker::RunRequestError, requests::RequestResult};

pub type RequestGraph = StableDiGraph<RequestNode, RequestEdgeType>;

#[derive(Debug)]
pub enum RequestNode {
  Error(RunRequestError),
  Root,
  Incomplete,
  Valid(RequestResult),
  Invalid,
  FileInvalidation,
}

#[derive(Debug)]
pub enum RequestEdgeType {
  SubRequest,
  FileChangeInvalidation,
}
