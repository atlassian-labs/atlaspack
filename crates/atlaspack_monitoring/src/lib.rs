//! This module configures tracing/sentry/crash-reporting.
//!
//! They are all disabled by default.
//!
//! Reporting should only be initialized once.
use std::sync::Mutex;
use std::time::Duration;

#[cfg(not(target_env = "musl"))]
pub use crash_reporter::CrashReporterOptions;

pub use sentry_integration::SentryOptions;
pub use tracer::TracerMode;

#[cfg(not(target_env = "musl"))]
mod crash_reporter;
mod from_env;
mod sentry_integration;
mod tracer;

pub static MONITORING_GUARD: Mutex<Option<MonitoringGuard>> = Mutex::new(None);

#[derive(Default)]
pub struct MonitoringGuard {
  sentry: Option<sentry::ClientInitGuard>,
  #[cfg(not(target_env = "musl"))]
  crash_handler: Option<crash_handler::CrashHandler>,
  #[allow(unused)]
  tracer: Option<tracer::Tracer>,
}

impl MonitoringGuard {
  #[cfg(not(target_env = "musl"))]
  pub fn crash_handler(&self) -> Option<&crash_handler::CrashHandler> {
    self.crash_handler.as_ref()
  }
}

#[derive(Debug)]
pub struct MonitoringOptions {
  pub tracing_options: Vec<TracerMode>,
  pub sentry_options: Option<SentryOptions>,
  #[cfg(not(target_env = "musl"))]
  pub crash_reporter_options: Option<CrashReporterOptions>,
}

impl MonitoringOptions {
  pub fn from_env() -> Result<Self, from_env::FromEnvError> {
    Ok(Self {
      tracing_options: TracerMode::from_env()?,
      sentry_options: SentryOptions::from_env()?,
      #[cfg(not(target_env = "musl"))]
      crash_reporter_options: CrashReporterOptions::from_env()?,
    })
  }
}

pub fn initialize_monitoring(options: MonitoringOptions) -> anyhow::Result<()> {
  let mut global = MONITORING_GUARD.lock().unwrap();
  if global.is_some() {
    tracing::warn!("Monitoring is getting set-up twice, this will no-op");
    return Ok(());
  }

  let sentry = options
    .sentry_options
    .map(sentry_integration::init_sentry)
    .transpose()?;

  // Order matters, tracer must be initialized after sentry
  let tracer = Some(tracer::Tracer::new(&options.tracing_options)?);

  #[cfg(not(target_env = "musl"))]
  let crash_handler = options
    .crash_reporter_options
    .map(crash_reporter::init_crash_reporter)
    .transpose()?;

  let guard = MonitoringGuard {
    sentry,
    #[cfg(not(target_env = "musl"))]
    crash_handler,
    tracer,
  };

  *global = Some(guard);

  Ok(())
}

pub fn initialize_from_env() -> anyhow::Result<()> {
  initialize_monitoring(MonitoringOptions::from_env()?)
}

/// We will block exiting the process for this period to try to flush metrics.
#[allow(unused)]
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

pub fn close_monitoring() {
  let monitoring_guard = MONITORING_GUARD.lock().unwrap();
  let Some(monitoring_guard) = monitoring_guard.as_ref() else {
    return;
  };

  if let Some(sentry_guard) = &monitoring_guard.sentry {
    tracing::debug!("Flushing sentry events");
    sentry_guard.flush(Some(CLOSE_TIMEOUT));
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_initialize_monitoring_multiple_times() {
    initialize_monitoring(MonitoringOptions {
      tracing_options: vec![TracerMode::Stdout],
      sentry_options: None,
      #[cfg(not(target_env = "musl"))]
      crash_reporter_options: None,
    })
    .unwrap();

    close_monitoring();

    initialize_monitoring(MonitoringOptions {
      tracing_options: vec![TracerMode::Stdout],
      sentry_options: None,
      #[cfg(not(target_env = "musl"))]
      crash_reporter_options: None,
    })
    .unwrap();

    close_monitoring();
  }
}
