//! This module configures `tracing_subscriber` to either write to a log file or standard output.
//!
//! Tracing is disabled by default.
use std::sync::Arc;
use std::sync::Mutex;

use crate::from_env::{optional_var, FromEnvError};
use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum TracerMode {
  /// Output the Tracer logs to Stdout
  Stdout,
  /// Output a Chrome profile
  Chrome,
}

impl TracerMode {
  pub fn from_env() -> Result<Vec<Self>, FromEnvError> {
    let Some(mode) = optional_var("ATLASPACK_TRACING_MODE") else {
      return Ok(vec![]);
    };

    let modes = mode.split(',').map(|s| s.trim()).collect::<Vec<&str>>();

    let mut tracer_modes = vec![];
    let mut used_modes = std::collections::HashSet::new();

    for mode in modes {
      match mode {
        "stdout" => {
          if used_modes.insert("stdout") {
            tracer_modes.push(Self::stdout());
          }
        }
        "chrome" => {
          if used_modes.insert("chrome") {
            tracer_modes.push(Self::chrome());
          }
        }
        "file" => {}
        value => {
          return Err(FromEnvError::InvalidKey(
            String::from("ATLASPACK_TRACING_MODE"),
            anyhow!("Invalid value: {}", value),
          ))
        }
      }
    }

    Ok(tracer_modes)
  }

  /// Default STDOUT configuration
  pub fn stdout() -> Self {
    Self::Stdout
  }

  pub fn chrome() -> Self {
    Self::Chrome
  }
}

enum TracerGuard {
  #[allow(unused)]
  WorkerGuard(WorkerGuard),
  #[allow(unused)]
  ChromeGuard(tracing_chrome::FlushGuard),
}

pub struct Tracer {
  #[allow(unused)]
  worker_guards: Arc<Mutex<Vec<TracerGuard>>>,
}

impl Tracer {
  pub fn new(options: &[TracerMode]) -> anyhow::Result<Self> {
    let mut worker_guards = vec![];

    // We will always write tracing to the log file
    let directory = std::env::temp_dir()
      .join("atlaspack_trace")
      .to_string_lossy()
      .to_string();
    let prefix = "atlaspack-tracing".to_string();
    let max_files = 4;
    let file_appender = tracing_appender::rolling::Builder::new()
      .rotation(tracing_appender::rolling::Rotation::HOURLY)
      .max_log_files(max_files as usize)
      .filename_prefix(&prefix)
      .build(&directory)
      .map_err(|err| anyhow::anyhow!(err))?;
    let (non_blocking, worker_guard) = tracing_appender::non_blocking(file_appender);

    let layer = tracing_subscriber::fmt::layer()
      .with_writer(non_blocking)
      .with_span_events(FmtSpan::CLOSE)
      .with_filter(EnvFilter::from_default_env());

    worker_guards.push(TracerGuard::WorkerGuard(worker_guard));

    let stdout_layer = if options
      .iter()
      .any(|mode| matches!(mode, TracerMode::Stdout))
    {
      let (non_blocking, worker_guard) = tracing_appender::non_blocking(std::io::stdout());
      let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(EnvFilter::from_default_env());

      worker_guards.push(TracerGuard::WorkerGuard(worker_guard));

      Some(stdout_layer)
    } else {
      None
    };

    let chrome_layer = if options
      .iter()
      .any(|mode| matches!(mode, TracerMode::Chrome))
    {
      let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
        .include_args(true)
        .build();

      worker_guards.push(TracerGuard::ChromeGuard(guard));

      Some(chrome_layer)
    } else {
      None
    };

    let sentry_layer = sentry_tracing::layer();

    let subscriber = Registry::default()
      .with(layer)
      .with(stdout_layer)
      .with(sentry_layer)
      .with(chrome_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    let tracer = Self {
      worker_guards: Arc::new(Mutex::new(worker_guards)),
    };

    Ok(tracer)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

  #[test]
  fn test_tracing_options_sets_to_none_if_no_mode_is_set() {
    let _guard = TEST_LOCK.lock();
    std::env::remove_var("ATLASPACK_TRACING_MODE");
    let options = TracerMode::from_env().unwrap();
    assert!(options.is_empty());
  }

  #[test]
  fn test_tracing_options_sets_to_file() {
    let _guard = TEST_LOCK.lock();
    std::env::set_var("ATLASPACK_TRACING_MODE", "stdout");
    let options = TracerMode::from_env().unwrap();
    assert!(options.contains(&TracerMode::stdout()));
  }
}
