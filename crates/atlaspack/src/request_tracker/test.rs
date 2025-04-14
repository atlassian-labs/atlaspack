use core::panic;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::requests::RequestResult;
use crate::test_utils::request_tracker;
use crate::WatchEvent;
use atlaspack_core::types::Invalidation;

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_request_chain() {
  let mut rt = request_tracker(Default::default());

  let request_c = TestRequest::new("C", &[]);
  let request_b = TestRequest::new("B", &[TestRequestType::Simple(request_c.clone())]);
  let request_a = TestRequest::new("A", &[TestRequestType::Simple(request_b.clone())]);

  let result = run_request(&mut rt, &request_a).await;

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(result[2], "C");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_request_caching() {
  let mut rt = request_tracker(Default::default());

  let request_c = TestRequest::new("C", &[]);
  let request_b = TestRequest::new("B", &[TestRequestType::Simple(request_c.clone())]);
  let request_a = TestRequest::new("A", &[TestRequestType::Simple(request_b.clone())]);

  let result = run_request(&mut rt, &request_a).await;

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(result[2], "C");

  let result = run_request(&mut rt, &request_a).await;

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(result[2], "C");
}

// SKIP: Always run requests / don't cache anything
// https://github.com/atlassian-labs/atlaspack/pull/364
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_single_request_execution() {
  let mut rt = request_tracker(Default::default());

  let request_a = TestRequest::new("A", &[]);

  let result = run_sub_request(&mut rt, &request_a).await;

  assert_eq!(result, "A");
  assert_eq!(request_a.run_count(), 1);

  let result = run_sub_request(&mut rt, &request_a).await;
  assert_eq!(result, "A");
  assert_eq!(request_a.run_count(), 1);
}

// SKIP: Always run requests / don't cache anything
// https://github.com/atlassian-labs/atlaspack/pull/364
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_single_execution_with_dependencies() {
  let mut rt = request_tracker(Default::default());

  let request_b = TestRequest::new("B", &[]);
  let request_a = TestRequest::new("A", &[TestRequestType::Simple(request_b.clone())]);

  let result = run_request(&mut rt, &request_a).await;

  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(request_a.run_count(), 1);
  assert_eq!(request_b.run_count(), 1);

  let result = run_request(&mut rt, &request_a).await;
  assert_eq!(result[0], "A");
  assert_eq!(result[1], "B");
  assert_eq!(request_a.run_count(), 1);
  assert_eq!(request_b.run_count(), 1);
}

async fn run_request(request_tracker: &mut RequestTracker, request: &TestRequest) -> Vec<String> {
  let RequestResult::TestMain(result) =
    &*request_tracker.run_request(request.clone()).await.unwrap()
  else {
    panic!("Unexpected result");
  };
  result.clone()
}

// SKIP: Always run requests / don't cache anything
// https://github.com/atlassian-labs/atlaspack/pull/364
async fn run_sub_request(request_tracker: &mut RequestTracker, request: &TestRequest) -> String {
  let RequestResult::TestSub(result) =
    &*request_tracker.run_request(request.clone()).await.unwrap()
  else {
    panic!("Unexpected result");
  };
  result.clone()
}

/// This is a universal "Request" that can be instructed
/// to run subrequests via the constructor
#[derive(Clone, Debug)]
enum TestRequestType {
  Simple(TestRequest),
  WithInvalidation(TestRequestWithInvalidation),
}

#[derive(Clone, Default, Debug)]
struct TestRequest {
  pub runs: Arc<AtomicUsize>,
  pub name: String,
  pub subrequests: Vec<TestRequestType>,
}

impl TestRequest {
  pub fn new<T: AsRef<str>>(name: T, subrequests: &[TestRequestType]) -> Self {
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

#[async_trait]
impl Request for TestRequest {
  async fn run(
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
      match subrequest {
        TestRequestType::Simple(req) => {
          let _ = request_context.queue_request(req, tx.clone());
        }
        TestRequestType::WithInvalidation(req) => {
          let _ = request_context.queue_request(req, tx.clone());
        }
      }
    }
    drop(tx);

    let mut results = vec![name];
    while let Ok(response) = rx.recv_timeout(Duration::from_secs(2)) {
      let (result, _id, _cached) = response?;
      match &*result {
        RequestResult::TestSub(result) => results.push(result.clone()),
        RequestResult::TestMain(sub_results) => results.extend(sub_results.clone()),
        _ => panic!("Unexpected request type"),
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

#[async_trait]
impl Request for TestChildRequest {
  async fn run(
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

#[async_trait]
impl Request for TestRequest2 {
  async fn run(
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
      let (result, _id, _cached) = response?;
      match &*result {
        RequestResult::TestSub(result) => responses.push(result.clone()),
        _ => todo!("unimplemented"),
      }
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::TestMain(responses),
      invalidations: vec![],
    })
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalidation_of_cached_results() {
  let mut rt = request_tracker(Default::default());

  // Create a request that depends on a file
  let request = TestRequestWithInvalidation::new("test", "test.txt");

  // First run should succeed and cache the result
  let result = rt.run_request(request.clone()).await.unwrap();
  assert!(matches!(&*result, RequestResult::TestSub(_)));

  // Simulate a file change event
  let events = vec![WatchEvent::Update(PathBuf::from("test.txt"))];
  let should_rebuild = rt.respond_to_fs_events(events);

  // Should indicate rebuild is needed
  assert!(should_rebuild);

  // Running the request again should execute it again rather than use cache
  let second_run = rt.run_request(request.clone()).await.unwrap();
  assert!(matches!(&*second_run, RequestResult::TestSub(_)));

  // Request should have run twice
  assert_eq!(request.run_count(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_selective_invalidation() {
  let mut rt = request_tracker(Default::default());

  // Create two independent requests watching different files
  let request_a = TestRequestWithInvalidation::new("A", "file_a.txt");
  let request_b = TestRequestWithInvalidation::new("B", "file_b.txt");

  // Run both requests initially
  let _ = rt.run_request(request_a.clone()).await.unwrap();
  let _ = rt.run_request(request_b.clone()).await.unwrap();

  // Simulate a change to only file_a.txt
  let events = vec![WatchEvent::Update(PathBuf::from("file_a.txt"))];
  rt.respond_to_fs_events(events);

  // Run both requests again
  let _ = rt.run_request(request_a.clone()).await.unwrap();
  let _ = rt.run_request(request_b.clone()).await.unwrap();

  // Request A should have run twice, but request B should have run only once
  assert_eq!(request_a.run_count(), 2);
  assert_eq!(request_b.run_count(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalidation_chain() {
  let mut rt = request_tracker(Default::default());

  // Create a chain of requests where each depends on the previous
  let request_c = TestRequestWithInvalidation::new("C", "file.txt");
  let request_b = TestRequest::new("B", &[TestRequestType::WithInvalidation(request_c.clone())]);
  let request_a = TestRequest::new("A", &[TestRequestType::Simple(request_b.clone())]);

  // Initial run
  let _ = rt.run_request(request_a.clone()).await.unwrap();

  // Simulate a change to the file that C depends on
  let events = vec![WatchEvent::Update(PathBuf::from("file.txt"))];
  let should_rebuild = rt.respond_to_fs_events(events);

  assert!(should_rebuild);

  // Run again
  let _ = rt.run_request(request_a.clone()).await.unwrap();

  // All requests in the chain should have run twice because C was invalidated
  assert_eq!(request_a.run_count(), 2);
  assert_eq!(request_b.run_count(), 2);
  assert_eq!(request_c.run_count(), 2);
}

// Add a new request type that includes file invalidation
#[derive(Clone, Debug)]
struct TestRequestWithInvalidation {
  runs: Arc<AtomicUsize>,
  name: String,
  watched_file: PathBuf,
}

impl TestRequestWithInvalidation {
  fn new<T: AsRef<str>>(name: T, watched_file: &str) -> Self {
    Self {
      runs: Default::default(),
      name: name.as_ref().to_string(),
      watched_file: PathBuf::from(watched_file),
    }
  }

  fn run_count(&self) -> usize {
    self.runs.load(Ordering::Relaxed)
  }
}

impl std::hash::Hash for TestRequestWithInvalidation {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

#[async_trait]
impl Request for TestRequestWithInvalidation {
  async fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    self.runs.fetch_add(1, Ordering::Relaxed);

    Ok(ResultAndInvalidations {
      result: RequestResult::TestSub(self.name.clone()),
      invalidations: vec![Invalidation::FileChange(self.watched_file.clone())],
    })
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_parallel_subrequests() {
  let sub_requests = 20;
  let result = request_tracker(Default::default())
    .run_request(TestRequest2 { sub_requests })
    .await
    .unwrap();

  match &*result {
    RequestResult::TestMain(responses) => {
      let expected: HashSet<String> = (0..sub_requests).map(|v| v.to_string()).collect();
      assert_eq!(HashSet::from_iter(responses.iter().cloned()), expected);
    }
    _ => {
      panic!("Request should pass");
    }
  }
}
