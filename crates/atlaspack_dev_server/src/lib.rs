use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use axum::{
  http::StatusCode,
  routing::{get, post, Route},
  Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Condvar;
use std::time::Duration;
use tower::Layer;
use tower_http::classify::MakeClassifier;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, info_span, Span};

#[derive(Parser, Deserialize, Debug, Serialize)]
pub struct Options {
  pub dist_dir: String,
}

#[derive(Clone)]
struct State {
  /// When true, the requests should wait until this is set to false
  ready: tokio::sync::watch::Receiver<bool>,
}

pub struct ServerHandle {
  join_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
  wait_tx: tokio::sync::watch::Sender<bool>,
}

impl ServerHandle {
  pub fn on_build_finished(&self) {
    self.wait_tx.send(true);
  }

  pub fn on_build_started(&self) {
    self.wait_tx.send(false);
  }

  pub fn stop(&self) {
    self.join_handle.abort();
  }
}

pub async fn run_server(options: Options) -> ServerHandle {
  // // initialize tracing
  // tracing_subscriber::fmt::init();

  // let options = Options::parse();
  let dist_dir = Path::new(&options.dist_dir);
  let (wait_tx, wait_rx) = tokio::sync::watch::channel(false);
  let state = State { ready: wait_rx };

  let app = Router::new()
    // `GET /` goes to `root`
    .nest(
      "/assets",
      serve_dir(&dist_dir.join("assets"), state.clone()),
    )
    .nest(
      "/fragments",
      serve_dir(&dist_dir.join("fragments"), state.clone()),
    )
    .layer(
      tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
          // Log the matched route's path (with placeholders not filled in).
          // Use request.uri() or OriginalUri if you want the real path.
          let uri = request.uri();

          info_span!(
              "http_request",
              method = ?request.method(),
              uri = ?uri,
              some_other_field = tracing::field::Empty,
          )
        })
        .on_response(|response: &Response, latency: Duration, _span: &Span| {
          info!(
            "{status} {latency:?}",
            status = response.status(),
            latency = latency,
          )
        }),
    );

  // run our app with hyper, listening globally on port 3000
  ServerHandle {
    join_handle: tokio::spawn(async move {
      let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
      tracing::info!("Listening on {}", listener.local_addr()?);
      axum::serve(listener, app).await?;
      Ok(())
    }),
    wait_tx,
  }
}

fn serve_dir(target_dir: &Path, state: State) -> Router {
  Router::new()
    // if state.wait is true, wait until it is set to false before responding
    .fallback_service(ServeDir::new(target_dir))
    .layer(axum::middleware::from_fn_with_state(
      state,
      wait_for_build_middleware,
    ))
}

async fn wait_for_build_middleware(
  mut state: axum::extract::State<State>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> impl IntoResponse {
  loop {
    let value: bool = *state.ready.borrow_and_update();
    info!("Build ready {value}");
    if value {
      return next.run(request).await;
    } else {
      info!("Waiting for build to finish");
      let change = state.ready.changed().await;
      if change.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Build failed").into_response();
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_start_server() {
    let _ = tracing_subscriber::fmt::try_init();

    let dist_dir = tempfile::tempdir().unwrap();
    // create assets directory with index.js
    let assets_dir = dist_dir.path().join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();
    std::fs::write(assets_dir.join("index.js"), "console.log('Hello, world!');").unwrap();

    let server = run_server(Options {
      dist_dir: dist_dir.path().to_string_lossy().to_string(),
    })
    .await;

    server.on_build_finished();

    let client = reqwest::Client::new();
    let response = client
      .get("http://localhost:3000/assets/index.js")
      .send()
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.text().await.unwrap(),
      "console.log('Hello, world!');"
    );
  }
}
