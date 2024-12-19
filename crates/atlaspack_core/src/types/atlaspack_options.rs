use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

use super::engines::Engines;
use super::EnvironmentContext;
use super::IncludeNodeModules;
use super::OutputFormat;
use super::TargetSourceMapOptions;

/// The options passed into Atlaspack either through the CLI or the programmatic API
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlaspackOptions {
  pub config: Option<String>,

  /// Path to the atlaspack core node_module. This will be used to resolve built-ins or runtime files.
  ///
  /// In the future this may be replaced with embedding those files into the rust binary.
  pub core_path: PathBuf,

  #[serde(default)]
  pub default_target_options: DefaultTargetOptions,

  pub entries: Vec<String>,
  pub env: Option<BTreeMap<String, String>>,

  #[serde(rename = "defaultConfig")]
  pub fallback_config: Option<String>,

  #[serde(default)]
  pub log_level: LogLevel,

  #[serde(default)]
  pub mode: BuildMode,

  pub threads: Option<usize>,

  pub targets: Option<Targets>,
}

#[derive(Clone, Debug, Hash, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Targets {
  Filter(Vec<String>),
  CustomTarget(BTreeMap<String, TargetDescriptor>),
}

#[derive(Debug, Clone, Deserialize, Hash, PartialEq, Serialize)]
pub enum SourceField {
  #[allow(unused)]
  Source(String),
  #[allow(unused)]
  Sources(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SourceMapField {
  Bool(bool),
  Options(TargetSourceMapOptions),
}

#[derive(Debug, Clone, Default, Deserialize, Hash, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TargetDescriptor {
  pub context: Option<EnvironmentContext>,
  pub dist_dir: Option<PathBuf>,
  pub dist_entry: Option<PathBuf>,
  pub engines: Option<Engines>,
  pub include_node_modules: Option<IncludeNodeModules>,
  pub is_library: Option<bool>,
  pub optimize: Option<bool>,
  pub output_format: Option<OutputFormat>,
  pub public_url: Option<String>,
  pub scope_hoist: Option<bool>,
  pub source: Option<SourceField>,
  pub source_map: Option<SourceMapField>,
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildMode {
  #[default]
  Development,
  Production,
  Other(String),
}

impl Display for BuildMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BuildMode::Development => write!(f, "development"),
      BuildMode::Production => write!(f, "production"),
      BuildMode::Other(mode) => write!(f, "{}", mode.to_lowercase()),
    }
  }
}

impl<'de> Deserialize<'de> for BuildMode {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;

    Ok(match s.as_str() {
      "development" => BuildMode::Development,
      "production" => BuildMode::Production,
      _ => BuildMode::Other(s),
    })
  }
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DefaultTargetOptions {
  pub dist_dir: Option<PathBuf>,
  pub engines: Engines,
  pub is_library: Option<bool>,
  pub output_format: Option<OutputFormat>,
  pub public_url: String,
  pub should_optimize: Option<bool>,
  pub should_scope_hoist: Option<bool>,
  pub source_maps: bool,
}

impl Default for DefaultTargetOptions {
  fn default() -> Self {
    Self {
      dist_dir: None,
      engines: Engines::default(),
      is_library: None,
      output_format: None,
      public_url: String::from("/"),
      should_optimize: None,
      should_scope_hoist: None,
      source_maps: false,
    }
  }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
  #[default]
  Error,
  Info,
  None,
  Verbose,
  Warn,
}
