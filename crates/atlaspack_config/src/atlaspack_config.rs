use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::diagnostic_error;
use atlaspack_core::types::AliasMap;
use atlaspack_core::types::DiagnosticError;
use serde::Deserialize;
use serde::Serialize;

use super::partial_atlaspack_config::PartialAtlaspackConfig;
use crate::map::NamedPipelinesMap;
use crate::map::PipelineMap;
use crate::map::PipelinesMap;

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNode {
  pub package_name: String,
  pub resolve_from: Arc<PathBuf>,
}

/// Represents a fully merged and validated .atlaspack_rc config
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AtlaspackConfig {
  pub bundler: PluginNode,
  pub compressors: PipelinesMap,
  pub namers: Vec<PluginNode>,
  pub optimizers: NamedPipelinesMap,
  pub packagers: PipelineMap,
  pub reporters: Vec<PluginNode>,
  pub resolvers: Vec<PluginNode>,
  pub runtimes: Vec<PluginNode>,
  pub transformers: NamedPipelinesMap,
  pub validators: PipelinesMap,
  pub unstable_alias: Option<AliasMap>,
}

impl TryFrom<PartialAtlaspackConfig> for AtlaspackConfig {
  type Error = DiagnosticError;

  fn try_from(config: PartialAtlaspackConfig) -> Result<Self, Self::Error> {
    let mut missing_phases = Vec::new();

    if config.bundler.is_none() {
      missing_phases.push(String::from("bundler"));
    }

    if config.namers.is_empty() {
      missing_phases.push(String::from("namers"));
    }

    if config.resolvers.is_empty() {
      missing_phases.push(String::from("resolvers"));
    }

    if !missing_phases.is_empty() {
      return Err(diagnostic_error!(
        "Missing plugins for the following phases: {:?}",
        missing_phases
      ));
    }

    Ok(AtlaspackConfig {
      bundler: config.bundler.unwrap(),
      compressors: PipelinesMap::new(config.compressors),
      namers: config.namers,
      optimizers: NamedPipelinesMap::new(config.optimizers),
      packagers: PipelineMap::new(config.packagers),
      reporters: config.reporters,
      resolvers: config.resolvers,
      runtimes: config.runtimes,
      transformers: NamedPipelinesMap::new(config.transformers),
      validators: PipelinesMap::new(config.validators),
      unstable_alias: config.unstable_alias,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  mod try_from {
    use super::*;

    #[test]
    fn returns_an_error_when_required_phases_are_optional() {
      assert_eq!(
        AtlaspackConfig::try_from(PartialAtlaspackConfig::default()).map_err(|e| e.to_string()),
        Err(
          diagnostic_error!(
            "Missing plugins for the following phases: {:?}",
            vec!("bundler", "namers", "resolvers")
          )
          .to_string()
        )
      );
    }
  }
}
