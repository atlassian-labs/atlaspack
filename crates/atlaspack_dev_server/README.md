# Atlaspack Dev Server

A development server built with Axum for the Atlaspack project.

## Features

- **Health Check Endpoint**: `GET /health` - Returns server status
- **API Status Endpoint**: `GET /api/status` - Returns build status and timestamp
- **Static File Serving**: `GET /static/*` - Serves files from `./dist` directory
- **CORS Support**: Configured to allow cross-origin requests
- **Logging**: Built-in request logging with tracing

## Usage

### As a Library

```rust
use atlaspack_dev_server::{DevServer, start_dev_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Option 1: Use the convenience function
    start_dev_server(Some(3000)).await?;
    
    // Option 2: Create a custom server
    let server = DevServer::new("127.0.0.1".to_string(), 8080);
    server.start().await?;
    
    Ok(())
}
```

### As a Binary

```bash
# Run the dev server on port 3000
cargo run --bin atlaspack_dev_server

# Or build and run
cargo build --release
./target/release/atlaspack_dev_server
```

## API Endpoints

- `GET /health` - Health check endpoint
  ```json
  {
    "status": "ok",
    "service": "atlaspack-dev-server"
  }
  ```

- `GET /api/status` - Build status endpoint
  ```json
  {
    "build_status": "ready",
    "timestamp": 1234567890
  }
  ```

- `GET /static/*` - Static file serving from `./dist` directory

## Dependencies

- `axum` - Web server framework
- `tokio` - Async runtime
- `serde` & `serde_json` - JSON serialization
- `tracing` & `tracing-subscriber` - Logging and observability
- `tower` & `tower-http` - Middleware and HTTP services

## Testing

```bash
cargo test
```