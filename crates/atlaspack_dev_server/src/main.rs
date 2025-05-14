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

#[derive(Parser)]
struct Options {
  dist_dir: String,
}

#[derive(Clone)]
struct State {
  /// When true, the requests should wait until this is set to false
  ready: tokio::sync::watch::Receiver<bool>,
}

#[tokio::main]
async fn main() {
  // initialize tracing
  tracing_subscriber::fmt::init();

  let options = Options::parse();
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
  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  tracing::info!("Listening on {}", listener.local_addr().unwrap());

  // tokio::spawn(async move {
  //   loop {
  //     info!("Simulating build...");
  //     wait_tx.send(false);
  //     tokio::time::sleep(Duration::from_secs(2)).await;
  //     info!("Build finished");
  //     wait_tx.send(true);
  //     tokio::time::sleep(Duration::from_secs(2)).await;
  //   }
  // });

  axum::serve(listener, app).await.unwrap();
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
      let _ = state.ready.changed().await;
    }
  }
}
