use crate::types::Dependency;
use async_trait::async_trait;
use std::sync::Arc;
use std::{fmt::Debug, path::PathBuf};

pub mod composite_reporter_plugin;

pub struct ResolvingEvent {
  pub dependency: Arc<Dependency>,
}

pub struct AssetBuildEvent {
  pub file_path: PathBuf,
}

pub enum BuildProgressEvent {
  Resolving(ResolvingEvent),
  Building(AssetBuildEvent),
}

// TODO Flesh these out
pub enum ReporterEvent {
  BuildStart,
  BuildProgress(BuildProgressEvent),
  BuildFailure,
  BuildSuccess,
  Log,
  Validation,
  WatchStart,
  WatchEnd,
}

/// Receives events from Atlaspack as they occur throughout the build process
///
/// For example, reporters may write status information to stdout, run a dev server, or generate a
/// bundle analysis report at the end of a build.
///
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
#[mockall::automock]
#[async_trait]
pub trait ReporterPlugin: Debug + Send + Sync {
  /// Processes the event from Atlaspack
  async fn report(&self, event: &ReporterEvent) -> Result<(), anyhow::Error>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestReporterPlugin {}

  #[async_trait]
  impl ReporterPlugin for TestReporterPlugin {
    async fn report(&self, _event: &ReporterEvent) -> Result<(), anyhow::Error> {
      todo!()
    }
  }

  #[test]
  fn can_be_defined_in_dyn_vec() {
    let reporters: Vec<Box<dyn ReporterPlugin>> = vec![Box::new(TestReporterPlugin {})];

    assert_eq!(reporters.len(), 1);
  }
}
