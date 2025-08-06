use std::net::SocketAddr;
use tracing::info;
use warp::Filter;

pub struct DevServer {
  port: u16,
  host: String,
}

impl DevServer {
  pub fn new(host: String, port: u16) -> Self {
    Self { host, port }
  }

  pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!(
      "Starting Atlaspack dev server on {}:{}",
      self.host, self.port
    );

    // Create basic routes
    let routes = self.create_routes();

    // Parse the socket address
    let addr: SocketAddr = format!("{}:{}", self.host, self.port).parse()?;

    info!("Dev server listening on http://{}", addr);

    // Start the server
    warp::serve(routes).run(addr).await;

    Ok(())
  }

  // New method for testing: start server and return the actual bound address
  pub async fn start_with_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    use std::net::TcpListener;

    let routes = self.create_routes();

    // If port is 0, find an available port
    let addr = if self.port == 0 {
      let listener = TcpListener::bind(format!("{}:0", self.host))?;
      let addr = listener.local_addr()?;
      drop(listener); // Close the listener so warp can bind to it
      addr
    } else {
      format!("{}:{}", self.host, self.port).parse()?
    };

    // Start the server in the background
    tokio::spawn(warp::serve(routes).run(addr));

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    Ok(addr)
  }

  fn create_routes(
    &self,
  ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let health = warp::path("health").and(warp::get()).map(|| {
      warp::reply::json(&serde_json::json!({
          "status": "ok",
          "service": "atlaspack-dev-server"
      }))
    });

    let static_files = warp::path("static").and(warp::fs::dir("./dist"));

    let api = warp::path("api").and(warp::path("status").and(warp::get()).map(|| {
      warp::reply::json(&serde_json::json!({
          "build_status": "ready",
          "timestamp": std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)
              .unwrap()
              .as_secs()
      }))
    }));

    let cors = warp::cors()
      .allow_any_origin()
      .allow_headers(vec!["content-type"])
      .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    // Combine all routes
    health
      .or(static_files)
      .or(api)
      .with(cors)
      .with(warp::log("atlaspack_dev_server"))
  }
}

// Convenience function to start a dev server with default settings
pub async fn start_dev_server(port: Option<u16>) -> Result<(), Box<dyn std::error::Error>> {
  let server = DevServer::new("127.0.0.1".to_string(), port.unwrap_or(3000));
  server.start().await
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use tempfile::TempDir;
  use tokio::time::sleep;

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
    let server = DevServer::new("127.0.0.1".to_string(), 0);
    let addr = server
      .start_with_addr()
      .await
      .expect("Failed to start server");

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore dir");

    (addr, temp_dir)
  }

  #[test]
  fn test_dev_server_creation() {
    let server = DevServer::new("localhost".to_string(), 8080);
    assert_eq!(server.host, "localhost");
    assert_eq!(server.port, 8080);
  }

  #[tokio::test]
  async fn test_health_endpoint() {
    let (addr, _temp_dir) = setup_test_server().await;
    let client = reqwest::Client::new();

    let response = client
      .get(&format!("http://{}/health", addr))
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
      .get(&format!("http://{}/api/status", addr))
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
      .get(&format!("http://{}/static/test.txt", addr))
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
      .get(&format!("http://{}/static/nonexistent.txt", addr))
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
      .get(&format!("http://{}/health", addr))
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
      .request(reqwest::Method::OPTIONS, &format!("http://{}/health", addr))
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
      .get(&format!("http://{}/nonexistent", addr))
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
      let addr = addr;
      let handle = tokio::spawn(async move {
        client
          .get(&format!("http://{}/health", addr))
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
      .get(&format!("http://{}/health", addr))
      .send()
      .await
      .expect("Failed to send GET request");
    assert_eq!(response.status(), 200);

    // Test POST to health endpoint (should fail - only GET allowed)
    let response = client
      .post(&format!("http://{}/health", addr))
      .send()
      .await
      .expect("Failed to send POST request");
    assert_eq!(response.status(), 405); // Method Not Allowed
  }
}
