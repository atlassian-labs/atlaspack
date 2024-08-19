use core::panic;
use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

use crate::requests::RequestResult;
use crate::test_utils::request_tracker;

use super::*;

#[test]
fn should_run_request() {
  let mut rt = request_tracker(Default::default());

  let request_c = TestRequest::new("C", &[]);
  let request_b = TestRequest::new("B", &[request_c.clone()]);
  let request_a = TestRequest::new("A", &[request_b.clone()]);

  let result = run_request(&mut rt, &request_a);

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(result[2], "C");
}

#[test]
fn should_reuse_previously_run_request() {
  let mut rt = request_tracker(Default::default());

  let request_c = TestRequest::new("C", &[]);
  let request_b = TestRequest::new("B", &[request_c.clone()]);
  let request_a = TestRequest::new("A", &[request_b.clone()]);

  let result = run_request(&mut rt, &request_a);

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(result[2], "C");

  let result = run_request(&mut rt, &request_a);

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(result[2], "C");
}

#[test]
fn should_run_request_once() {
  let mut rt = request_tracker(Default::default());

  let request_a = TestRequest::new("A", &[]);

  let result = run_sub_request(&mut rt, &request_a);

  assert_eq!(result, "A");
  assert_eq!(request_a.run_count(), 1);

  let result = run_sub_request(&mut rt, &request_a);
  assert_eq!(result, "A");
  assert_eq!(request_a.run_count(), 1);
}

#[test]
fn should_run_request_once_2() {
  let mut rt = request_tracker(Default::default());

  let request_b = TestRequest::new("B", &[]);
  let request_a = TestRequest::new("A", &[request_b.clone()]);

  let result = run_request(&mut rt, &request_a);

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(request_a.run_count(), 1);
  assert_eq!(request_b.run_count(), 1);

  let result = run_request(&mut rt, &request_a);
  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(request_a.run_count(), 1);
  assert_eq!(request_b.run_count(), 1);
}

fn run_request(request_tracker: &mut RequestTracker, request: &TestRequest) -> Vec<String> {
  let RequestResult::TestMain(result) = request_tracker.run_request(request.clone()).unwrap()
  else {
    panic!("Unexpected result");
  };
  result
}

fn run_sub_request(request_tracker: &mut RequestTracker, request: &TestRequest) -> String {
  let RequestResult::TestSub(result) = request_tracker.run_request(request.clone()).unwrap() else {
    panic!("Unexpected result");
  };
  result
}

/// This is a universal "Request" that can be instructed
/// to run subrequests via the constructor
#[derive(Clone, Default, Debug)]
pub struct TestRequest {
  pub runs: Arc<AtomicUsize>,
  pub name: String,
  pub subrequests: Vec<TestRequest>,
}

impl TestRequest {
  pub fn new<T: AsRef<str>>(name: T, subrequests: &[TestRequest]) -> Self {
    Self {
      runs: Default::default(),
      name: name.as_ref().to_string(),
      subrequests: subrequests.to_owned(),
    }
  }

  pub fn run_count(&self) -> usize {
    self.runs.load(Ordering::Relaxed)
  }
}

impl std::hash::Hash for TestRequest {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

impl Request for TestRequest {
  fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    self.runs.fetch_add(1, Ordering::Relaxed);

    let name = self.name.clone();

    let mut subrequests = self.subrequests.clone();

    if subrequests.is_empty() {
      return Ok(ResultAndInvalidations {
        result: RequestResult::TestSub(name),
        invalidations: vec![],
      });
    }

    let (tx, rx) = channel();

    while let Some(subrequest) = subrequests.pop() {
      let req = subrequest.clone();
      let _ = request_context.queue_request(req, tx.clone());
    }
    drop(tx);

    let mut results = vec![name];
    while let Ok(response) = rx.recv_timeout(Duration::from_secs(2)) {
      match response {
        Ok((RequestResult::TestSub(result), _id)) => results.push(result),
        Ok((RequestResult::TestMain(sub_results), _id)) => results.extend(sub_results),
        a => todo!("{:?}", a),
      }
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::TestMain(results),
      invalidations: vec![],
    })
  }
}

#[derive(Debug, Hash)]
struct TestChildRequest {
  count: u32,
}
impl Request for TestChildRequest {
  fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    Ok(ResultAndInvalidations {
      result: RequestResult::TestSub(self.count.to_string()),
      invalidations: vec![],
    })
  }
}
#[derive(Debug, Hash)]
struct TestRequest2 {
  sub_requests: u32,
}
impl Request for TestRequest2 {
  fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let (tx, rx) = channel();

    for count in 0..self.sub_requests {
      let _ = request_context.queue_request(TestChildRequest { count }, tx.clone());
    }
    drop(tx);

    let mut responses = Vec::new();
    while let Ok(response) = rx.recv_timeout(Duration::from_secs(2)) {
      match response {
        Ok((RequestResult::TestSub(result), _idd)) => responses.push(result),
        _ => todo!("unimplemented"),
      }
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::TestMain(responses),
      invalidations: vec![],
    })
  }
}

#[test]
fn test_queued_subrequests() {
  let sub_requests = 20;
  let result = request_tracker(Default::default()).run_request(TestRequest2 { sub_requests });

  match result {
    Ok(RequestResult::TestMain(responses)) => {
      let expected: HashSet<String> = (0..sub_requests).map(|v| v.to_string()).collect();
      assert_eq!(HashSet::from_iter(responses.iter().cloned()), expected);
    }
    _ => {
      panic!("Request should pass");
    }
  }
}
