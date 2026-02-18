use core::panic;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::time::Duration;

use async_trait::async_trait;

use crate::WatchEvent;
use crate::requests::RequestResult;
use crate::test_utils::request_tracker;
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
  let response = request_tracker.run_request(request.clone()).await.unwrap();
  let RequestResult::TestMain(result) = response.as_ref() else {
    panic!("Unexpected result");
  };
  result.clone()
}

// SKIP: Always run requests / don't cache anything
// https://github.com/atlassian-labs/atlaspack/pull/364
#[allow(dead_code)]
async fn run_sub_request(request_tracker: &mut RequestTracker, request: &TestRequest) -> String {
  let response = request_tracker.run_request(request.clone()).await.unwrap();
  let RequestResult::TestSub(result) = response.as_ref() else {
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
      match response {
        Ok((result, _id, _cached)) => match result.as_ref() {
          RequestResult::TestSub(r) => results.push(r.clone()),
          RequestResult::TestMain(sub_results) => results.extend(sub_results.clone()),
          _ => todo!(),
        },
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
      match response {
        Ok((result, _id, _cached)) => match result.as_ref() {
          RequestResult::TestSub(r) => responses.push(r.clone()),
          _ => todo!("unimplemented"),
        },
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
  assert!(matches!(result.as_ref(), RequestResult::TestSub(_)));

  // Simulate a file change event
  let events = vec![WatchEvent::Update(PathBuf::from("test.txt"))];
  let should_rebuild = rt.respond_to_fs_events(events);

  // Should indicate rebuild is needed
  assert!(should_rebuild);

  // Running the request again should execute it again rather than use cache
  let second_run = rt.run_request(request.clone()).await.unwrap();
  assert!(matches!(second_run.as_ref(), RequestResult::TestSub(_)));

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

  // Request A should have run twice, request B should have run once
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
    .await;

  match result {
    Ok(result) => match result.as_ref() {
      RequestResult::TestMain(responses) => {
        let expected: HashSet<String> = (0..sub_requests).map(|v| v.to_string()).collect();
        assert_eq!(HashSet::from_iter(responses.iter().cloned()), expected);
      }
      _ => {
        panic!("Request should pass");
      }
    },
    _ => {
      panic!("Request should pass");
    }
  }
}

/// Test request that uses the new execute_request method
#[derive(Clone, Default, Debug)]
struct TestExecuteRequest {
  pub runs: Arc<AtomicUsize>,
  pub name: String,
}

impl TestExecuteRequest {
  pub fn new<T: AsRef<str>>(name: T) -> Self {
    Self {
      runs: Default::default(),
      name: name.as_ref().to_string(),
    }
  }

  pub fn run_count(&self) -> usize {
    self.runs.load(Ordering::Relaxed)
  }
}

impl std::hash::Hash for TestExecuteRequest {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

#[async_trait]
impl Request for TestExecuteRequest {
  async fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    self.runs.fetch_add(1, Ordering::Relaxed);

    // Test the new execute_request method by running a child request
    let child_request = TestRequest::new("child", &[]);
    let (result, _request_id, _cached) = request_context.execute_request(child_request).await?;

    match result.as_ref() {
      RequestResult::TestSub(name) => {
        assert_eq!(name, "child");
        Ok(ResultAndInvalidations {
          result: RequestResult::TestSub(self.name.clone()),
          invalidations: vec![],
        })
      }
      _ => Err(anyhow::anyhow!("Unexpected result type").into()),
    }
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_execute_request() {
  let mut graph = request_tracker(Default::default());

  let parent_request = TestExecuteRequest::new("parent");
  let result = graph.run_request(parent_request.clone()).await;

  assert!(result.is_ok());
  assert_eq!(parent_request.run_count(), 1);

  let result = result.unwrap();
  match result.as_ref() {
    RequestResult::TestSub(name) => {
      assert_eq!(name, "parent");
    }
    _ => panic!("Unexpected result type"),
  }
}

/// A child request whose Hash (and thus request ID) changes between the first
/// and subsequent runs. On the first run it hashes as "child-v1", on the second
/// as "child-v2". This simulates the real-world bug where `try_reuse_asset_graph`
/// reconstructs an `AssetRequest` with different field values (e.g. `side_effects`)
/// than the original, producing a different request ID.
#[derive(Clone, Debug)]
struct TestChildRequestWithChangingId {
  runs: Arc<AtomicUsize>,
  watched_file: PathBuf,
}

impl TestChildRequestWithChangingId {
  fn new(watched_file: &str) -> Self {
    Self {
      runs: Arc::new(AtomicUsize::new(0)),
      watched_file: PathBuf::from(watched_file),
    }
  }

  fn run_count(&self) -> usize {
    self.runs.load(Ordering::Relaxed)
  }
}

impl std::hash::Hash for TestChildRequestWithChangingId {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    let run_count = self.runs.load(Ordering::Relaxed);
    if run_count == 0 {
      "child-v1".hash(state);
    } else {
      "child-v2".hash(state);
    }
  }
}

#[async_trait]
impl Request for TestChildRequestWithChangingId {
  async fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    self.runs.fetch_add(1, Ordering::Relaxed);
    Ok(ResultAndInvalidations {
      result: RequestResult::TestSub("child".to_string()),
      invalidations: vec![Invalidation::FileChange(self.watched_file.clone())],
    })
  }
}

/// A parent request that uses execute_request to run a child whose ID changes
/// between runs. This simulates AssetGraphRequest calling try_reuse_asset_graph
/// which reconstructs AssetRequests with different field values.
#[derive(Clone, Debug)]
struct TestParentWithChangingChild {
  runs: Arc<AtomicUsize>,
  child: TestChildRequestWithChangingId,
}

impl TestParentWithChangingChild {
  fn new(child: TestChildRequestWithChangingId) -> Self {
    Self {
      runs: Arc::new(AtomicUsize::new(0)),
      child,
    }
  }

  fn run_count(&self) -> usize {
    self.runs.load(Ordering::Relaxed)
  }
}

impl std::hash::Hash for TestParentWithChangingChild {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    "parent-with-changing-child".hash(state);
  }
}

#[async_trait]
impl Request for TestParentWithChangingChild {
  async fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    self.runs.fetch_add(1, Ordering::Relaxed);

    let (_result, _request_id, _cached) =
      request_context.execute_request(self.child.clone()).await?;

    Ok(ResultAndInvalidations {
      result: RequestResult::TestSub("parent".to_string()),
      invalidations: vec![],
    })
  }
}

/// Regression test for the infinite incremental rebuild loop bug.
///
/// When a parent request (like AssetGraphRequest) re-runs a child request
/// (like AssetRequest) with a different ID after file invalidation, the
/// original child's node becomes permanently orphaned in `invalid_nodes`.
/// This causes `respond_to_fs_events` to always return `true`, even for
/// unrelated file events, creating an infinite rebuild loop.
///
/// Real-world scenario: The resolver sets `side_effects=false` (from package.json),
/// but a transformer overrides it to `true`. On incremental rebuild,
/// `try_reuse_asset_graph` reads the transformer's output (`side_effects=true`)
/// and creates a new AssetRequest with a different hash/ID. The original
/// invalidated AssetRequest node is never cleared from `invalid_nodes`.
#[tokio::test(flavor = "multi_thread")]
async fn test_orphaned_invalid_node_causes_perpetual_rebuild() {
  let mut rt = request_tracker(Default::default());

  let child = TestChildRequestWithChangingId::new("watched.txt");
  let parent = TestParentWithChangingChild::new(child.clone());

  // Step 1: Initial build. The child runs as "child-v1" and registers
  // a file invalidation on "watched.txt".
  let result = rt.run_request(parent.clone()).await.unwrap();
  assert!(matches!(result.as_ref(), RequestResult::TestSub(_)));
  assert_eq!(parent.run_count(), 1);
  assert_eq!(child.run_count(), 1);

  // Step 2: Simulate a file change to "watched.txt". This should invalidate
  // both the parent and the child ("child-v1") nodes.
  let events = vec![WatchEvent::Update(PathBuf::from("watched.txt"))];
  let should_rebuild = rt.respond_to_fs_events(events);
  assert!(should_rebuild, "File change should trigger rebuild");

  // Step 3: Run the parent again. Now the child hashes as "child-v2"
  // (different ID), simulating the side_effects mismatch.
  let result = rt.run_request(parent.clone()).await.unwrap();
  assert!(matches!(result.as_ref(), RequestResult::TestSub(_)));
  assert_eq!(parent.run_count(), 2);
  assert_eq!(child.run_count(), 2);

  // Step 4: At the RequestTracker level, the orphaned node still exists.
  // The child was re-run under "child-v2", but the original "child-v1"
  // node remains in invalid_nodes because prepare_request only removes
  // the node for the request ID that is actually re-run.
  let orphaned_count = rt.get_invalid_nodes().count();
  assert_eq!(
    orphaned_count, 1,
    "Expected 1 orphaned invalid node after rebuild with changed request ID, \
     found {}.",
    orphaned_count
  );

  // Step 5: The fix — build_asset_graph() calls clear_invalid_nodes()
  // after a successful top-level build to remove orphans.
  rt.clear_invalid_nodes();
  assert_eq!(
    rt.get_invalid_nodes().count(),
    0,
    "clear_invalid_nodes should remove all orphaned entries"
  );

  // Step 6: After clearing, unrelated file events should NOT trigger
  // a rebuild.
  let unrelated_events = vec![WatchEvent::Update(PathBuf::from("unrelated.txt"))];
  let should_rebuild_again = rt.respond_to_fs_events(unrelated_events);
  assert!(
    !should_rebuild_again,
    "Unrelated file events should NOT trigger a rebuild after clearing invalid nodes."
  );
}
