use async_trait::async_trait;
use axum::{
  body::Body,
  extract::{Request, State},
  response::{IntoResponse, Json, Response},
  routing::get,
  Router,
};
use reqwest::Url;
use serde_json::{json, Value};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::{
  cors::CorsLayer,
  services::{ServeDir, ServeFile},
  trace::TraceLayer,
};
use tracing::info;

#[async_trait]
pub trait DevServerDataProvider: std::fmt::Debug {
  async fn get_html_bundle_file_paths(&self) -> anyhow::Result<Vec<String>>;
  async fn request_bundle(&self, requested_path: String) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct DevServerOptions {
  pub host: String,
  pub port: u16,
  pub public_url: Option<String>,
  pub dist_dir: PathBuf,
  pub data_provider: Box<dyn DevServerDataProvider + Send + Sync>,
}

pub struct DevServer {
  state: Arc<DevServerState>,
  task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

struct DevServerState {
  root_path: String,
  options: DevServerOptions,
}

impl DevServer {
  pub fn new(options: DevServerOptions) -> Self {
    let root_path = options
      .public_url
      .as_ref()
      .map(|url| {
        Url::parse(url)
          .map(|url| url.path().to_string())
          .unwrap_or_else(|_| "/".to_string())
      })
      .unwrap_or_else(|| "/".to_string());

    info!("Dev server created with options {:?}", options);

    Self {
      state: Arc::new(DevServerState { root_path, options }),
      task: Mutex::new(None),
    }
  }

  pub async fn start(&self) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt::try_init();

    let app = self.create_app();

    let addr = format!("{}:{}", self.state.options.host, self.state.options.port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;

    let task = tokio::spawn(async move {
      axum::serve(listener, app).await.unwrap();
    });
    info!("Dev server started on {}", addr);

    self.task.lock().await.replace(task);

    Ok(addr)
  }

  pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
    let task = self.task.lock().await.take();
    if let Some(task) = task {
      task.abort();
    }
    Ok(())
  }

  fn create_app(&self) -> Router {
    Router::new()
      .route("/", get(index_handler))
      .route("/__atlaspack__/api/health", get(health_handler))
      .route("/__atlaspack__/api/status", get(api_status_handler))
      .fallback(get_handler)
      .layer(
        ServiceBuilder::new()
          .layer(TraceLayer::new_for_http())
          .layer(
            CorsLayer::new()
              .allow_origin(tower_http::cors::Any)
              .allow_headers(tower_http::cors::Any)
              .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
              ]),
          ),
      )
      .with_state(self.state.clone())
  }
}

async fn index_handler(
  State(state): State<Arc<DevServerState>>,
  request: Request<Body>,
) -> Response {
  let mut serve_file = ServeFile::new(state.options.dist_dir.join("index.html"));
  let result = serve_file
    .try_call(request)
    .await
    .map(|r| r.into_response());

  result_to_response(result)
}

async fn get_handler(State(state): State<Arc<DevServerState>>, request: Request<Body>) -> Response {
  println!("get_handler {:?}", request.uri());
  println!("state.root_path {:?}", state.root_path);
  println!("state.options.dist_dir {:?}", state.options.dist_dir);

  let mut serve_dir = ServeDir::new(state.options.dist_dir.clone());
  let path = request.uri().path();
  info!("request: {:?}", path);

  if path.starts_with(&state.root_path) {
    let request_result = state
      .options
      .data_provider
      .request_bundle(path[1..].to_string())
      .await;

    println!("request_result {:?}", request_result);

    let result = serve_dir.try_call(request).await.map(|r| r.into_response());

    return result_to_response(result);
  }

  let response = Response::builder()
    .status(404)
    .body(Body::from("Not Found"))
    .map(|r| r.into_response());
  result_to_response(response)
}

fn result_to_response<E: std::error::Error>(response: Result<Response, E>) -> Response {
  response.unwrap_or_else(|error| {
    Response::builder()
      .status(500)
      .body(Body::from(format!("Internal server error: {}", error)))
      .unwrap()
  })
}

// Handler functions
async fn health_handler() -> Json<Value> {
  Json(json!({
    "status": "ok",
    "service": "atlaspack-dev-server"
  }))
}

async fn api_status_handler() -> Json<Value> {
  Json(json!({
    "build_status": "ready",
    "timestamp": std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs()
  }))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use tempfile::TempDir;
  use tokio::time::sleep;

  #[derive(Debug)]
  struct MockDevServerDataProvider {}

  #[async_trait]
  impl DevServerDataProvider for MockDevServerDataProvider {
    async fn get_html_bundle_file_paths(&self) -> anyhow::Result<Vec<String>> {
      Ok(vec![])
    }

    async fn request_bundle(&self, _requested_path: String) -> anyhow::Result<()> {
      Ok(())
    }
  }

  async fn setup_test_server() -> (SocketAddr, TempDir) {
    // Create a temporary directory for static files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let dist_path = temp_dir.path().join("dist");
    std::fs::create_dir_all(&dist_path).expect("Failed to create dist dir");

    // Create a test file in the dist directory
    let test_file_path = dist_path.join("test.txt");
    std::fs::write(&test_file_path, "Hello from static file!").expect("Failed to write test file");

    // Change to the temp directory so the server can find ./dist
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

    // Create server with port 0 to get a random available port
    let server = DevServer::new(DevServerOptions {
      host: "127.0.0.1".to_string(),
      port: 0,
      dist_dir: PathBuf::from("./dist"),
      public_url: None,
      data_provider: Box::new(MockDevServerDataProvider {}),
    });
    let addr = server.start().await.expect("Failed to start server");

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore dir");

    (addr, temp_dir)
  }

  #[test]
  fn test_dev_server_creation() {
    let server = DevServer::new(DevServerOptions {
      host: "localhost".to_string(),
      port: 8080,
      dist_dir: PathBuf::from("./dist"),
      public_url: None,
      data_provider: Box::new(MockDevServerDataProvider {}),
    });
    assert_eq!(server.state.options.host, "localhost");
    assert_eq!(server.state.options.port, 8080);
  }

  #[tokio::test]
  async fn test_health_endpoint() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    let response = client
      .get(&format!("http://{}/__atlaspack__/api/health", addr))
      .send()
      .await
      .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response
      .json()
      .await
      .expect("Failed to parse JSON response");

    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "atlaspack-dev-server");
  }

  #[tokio::test]
  async fn test_api_status_endpoint() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    let response = client
      .get(&format!("http://{}/__atlaspack__/api/status", addr))
      .send()
      .await
      .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response
      .json()
      .await
      .expect("Failed to parse JSON response");

    assert_eq!(json["build_status"], "ready");
    assert!(json["timestamp"].is_number());

    // Verify timestamp is reasonable (within last minute)
    let timestamp = json["timestamp"]
      .as_u64()
      .expect("Timestamp should be a number");
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs();
    assert!(timestamp <= now);
    assert!(timestamp > now - 60); // Within last minute
  }

  #[tokio::test]
  async fn test_static_file_serving() {
    let (addr, temp_dir) = setup_test_server().await;

    // Change to temp directory for this test
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

    let client = reqwest::Client::new();

    let response = client
      .get(&format!("http://{}/test.txt", addr))
      .send()
      .await
      .expect("Failed to send request");

    // Restore directory
    std::env::set_current_dir(original_dir).expect("Failed to restore dir");

    assert_eq!(response.status(), 200);

    let text = response.text().await.expect("Failed to get response text");

    assert_eq!(text, "Hello from static file!");
  }

  #[tokio::test]
  async fn test_static_file_not_found() {
    let (addr, temp_dir) = setup_test_server().await;

    // Change to temp directory for this test
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

    let client = reqwest::Client::new();

    let response = client
      .get(&format!("http://{}/nonexistent.txt", addr))
      .send()
      .await
      .expect("Failed to send request");

    // Restore directory
    std::env::set_current_dir(original_dir).expect("Failed to restore dir");

    assert_eq!(response.status(), 404);
  }

  #[tokio::test]
  async fn test_cors_headers() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    // Send a request with Origin header to trigger CORS
    let response = client
      .get(&format!("http://{}/__atlaspack__/api/health", addr))
      .header("Origin", "http://localhost:3000")
      .send()
      .await
      .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    // Check CORS headers
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
  }

  #[tokio::test]
  async fn test_cors_preflight() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    // Send a preflight OPTIONS request
    let response = client
      .request(
        reqwest::Method::OPTIONS,
        &format!("http://{}/__atlaspack__/api/health", addr),
      )
      .header("Origin", "http://localhost:3000")
      .header("Access-Control-Request-Method", "GET")
      .send()
      .await
      .expect("Failed to send preflight request");

    assert_eq!(response.status(), 200);

    // Check CORS preflight headers
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert!(headers.contains_key("access-control-allow-methods"));
    assert!(headers.contains_key("access-control-allow-headers"));
  }

  #[tokio::test]
  async fn test_nonexistent_endpoint() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    let response = client
      .get(&format!("http://{}/__atlaspack__/api/nonexistent", addr))
      .send()
      .await
      .expect("Failed to send request");

    assert_eq!(response.status(), 404);
  }

  #[tokio::test]
  async fn test_multiple_concurrent_requests() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    // Send multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..10 {
      let client = client.clone();
      let addr = addr.clone();
      let handle = tokio::spawn(async move {
        client
          .get(&format!("http://{}/__atlaspack__/api/health", addr))
          .send()
          .await
          .expect("Failed to send request")
      });
      handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
      let response = handle.await.expect("Task failed");
      assert_eq!(response.status(), 200);
    }
  }

  #[tokio::test]
  async fn test_different_http_methods() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    // Test GET (should work)
    let response = client
      .get(&format!("http://{}/__atlaspack__/api/health", addr))
      .send()
      .await
      .expect("Failed to send GET request");
    assert_eq!(response.status(), 200);

    // Test POST to health endpoint (should fail - only GET allowed)
    let response = client
      .post(&format!("http://{}/__atlaspack__/api/health", addr))
      .send()
      .await
      .expect("Failed to send POST request");
    assert_eq!(response.status(), 405); // Method Not Allowed
  }
}
