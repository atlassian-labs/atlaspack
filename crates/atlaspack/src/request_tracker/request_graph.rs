use petgraph::stable_graph::StableDiGraph;

use crate::request_tracker::{ResultAndInvalidations, RunRequestError};

pub type RequestGraph = StableDiGraph<RequestNode, RequestEdgeType>;

#[derive(Debug)]
pub enum RequestNode {
  Error(RunRequestError),
  Root,
  Incomplete,
  Valid(ResultAndInvalidations),
}

#[derive(Debug)]
pub enum RequestEdgeType {
  SubRequest,
}
