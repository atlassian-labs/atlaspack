use std::collections::BTreeMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;

pub use output_format::OutputFormat;

use crate::hash::IdentifierHasher;

use super::source::SourceLocation;

use self::engines::Engines;

pub mod browsers;
pub mod engines;
mod output_format;
pub mod version;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EnvironmentId(pub NonZeroU32);

/// The environment the built code will run in
///
/// This influences how Atlaspack compiles your code, including what syntax to transpile.
///
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
  /// The environment the output should run in
  pub context: EnvironmentContext,

  /// The engines supported by the environment
  pub engines: Engines,

  /// Describes which node_modules should be included in the output
  pub include_node_modules: IncludeNodeModules,

  /// Whether this is a library build
  ///
  /// Treats the target as a library that would be published to npm and consumed by another tool,
  /// rather than used directly in a browser or other target environment.
  ///
  /// Library targets must enable scope hoisting, and use a non-global output format.
  ///
  pub is_library: bool,

  pub loc: Option<SourceLocation>,

  /// Determines what type of module to output
  pub output_format: OutputFormat,

  /// Determines whether scope hoisting should be enabled
  ///
  /// By default, scope hoisting is enabled for production builds.
  ///
  pub should_scope_hoist: bool,

  /// Determines whether the output should be optimised
  ///
  /// The exact behavior of this flag is determined by plugins. By default, optimization is
  /// enabled during production builds for application targets.
  ///
  pub should_optimize: bool,

  /// Configures source maps, which are enabled by default
  pub source_map: Option<TargetSourceMapOptions>,

  pub source_type: SourceType,
}

pub fn create_environment_id(
  context: &EnvironmentContext,
  engines: &Engines,
  include_node_modules: &IncludeNodeModules,
  output_format: &OutputFormat,
  source_type: &SourceType,
  is_library: &bool,
  should_optimize: &bool,
  should_scope_hoist: &bool,
  source_map: &Option<TargetSourceMapOptions>,
) -> String {
  let mut hasher = IdentifierHasher::new();
  context.hash(&mut hasher);
  engines.hash(&mut hasher);
  include_node_modules.hash(&mut hasher);
  output_format.hash(&mut hasher);
  source_type.hash(&mut hasher);
  is_library.hash(&mut hasher);
  should_optimize.hash(&mut hasher);
  should_scope_hoist.hash(&mut hasher);
  flatten_source_map(source_map).hash(&mut hasher);
  let hash = hasher.finish(); // We can simply expose this as a nº too
  format!("{:016x}", hash)
}

impl Environment {
  pub fn id(&self) -> String {
    let s = create_environment_id(
      &self.context,
      &self.engines,
      &self.include_node_modules,
      &self.output_format,
      &self.source_type,
      &self.is_library,
      &self.should_optimize,
      &self.should_scope_hoist,
      &self.source_map,
    );
    s
  }
}

/// The environment the output should run in
///
/// This informs Atlaspack what environment-specific APIs are available.
///
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentContext {
  #[default]
  Browser,
  ElectronMain,
  ElectronRenderer,
  Node,
  ServiceWorker,
  WebWorker,
  Worklet,
}

impl EnvironmentContext {
  pub fn is_node(&self) -> bool {
    use EnvironmentContext::*;
    matches!(self, Node | ElectronMain | ElectronRenderer)
  }

  pub fn is_browser(&self) -> bool {
    use EnvironmentContext::*;
    matches!(
      self,
      Browser | WebWorker | ServiceWorker | Worklet | ElectronRenderer
    )
  }

  pub fn is_worker(&self) -> bool {
    use EnvironmentContext::*;
    matches!(self, WebWorker | ServiceWorker)
  }

  pub fn is_worklet(&self) -> bool {
    use EnvironmentContext::*;
    matches!(self, Worklet)
  }

  pub fn is_electron(&self) -> bool {
    use EnvironmentContext::*;
    matches!(self, ElectronMain | ElectronRenderer)
  }
}

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum IncludeNodeModules {
  Bool(bool),
  Array(Vec<String>),
  // We can't hash a HashMap because we need to iterate in order
  Map(BTreeMap<String, bool>),
}

impl Default for IncludeNodeModules {
  fn default() -> Self {
    IncludeNodeModules::Bool(true)
  }
}

impl From<EnvironmentContext> for IncludeNodeModules {
  fn from(context: EnvironmentContext) -> Self {
    match context {
      EnvironmentContext::Browser => IncludeNodeModules::Bool(true),
      EnvironmentContext::ServiceWorker => IncludeNodeModules::Bool(true),
      EnvironmentContext::WebWorker => IncludeNodeModules::Bool(true),
      _ => IncludeNodeModules::Bool(false),
    }
  }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SourceType {
  #[default]
  #[serde(rename = "module")]
  Module,
  #[serde(rename = "script")]
  Script,
}

/// Source map options for the target output
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSourceMapOptions {
  /// Inlines the source map as a data URL into the bundle, rather than link to it as a separate output file
  #[serde(skip_serializing_if = "Option::is_none")]
  inline: Option<bool>,

  /// Inlines the original source code into the source map, rather than loading them from the source root
  ///
  /// This is set to true by default when building browser targets for production.
  ///
  #[serde(skip_serializing_if = "Option::is_none")]
  inline_sources: Option<bool>,

  /// The URL to load the original source code from
  ///
  /// This is set automatically in development when using the builtin Atlaspack development server.
  /// Otherwise, it defaults to a relative path to the bundle from the project root.
  ///
  #[serde(skip_serializing_if = "Option::is_none")]
  source_root: Option<String>,
}

/// We must consider { None, None, None } equal to None for all intents and purposes
pub fn flatten_source_map(source_map: &Option<TargetSourceMapOptions>) -> TargetSourceMapOptions {
  source_map.clone().unwrap_or_default()
}

#[cfg(test)]
mod test {
  use std::str::FromStr;

  use version::Version;

  use super::*;

  // This is here to check that the default environment hash will match
  // the one in Node.js - packages/core/core/test/Environment.test.js
  #[test]
  fn test_environment() {
    tracing_subscriber::fmt::init();
    let environment = Environment::default();
    let id = environment.id();
    assert_eq!(id, "bb871c88ce058724");

    let environment = Environment {
      context: EnvironmentContext::Node,
      engines: Engines {
        browsers: None,
        node: Some(Version::from_str("8.0.0").unwrap()),
        ..Default::default()
      },
      output_format: OutputFormat::CommonJS,
      ..Default::default()
    };
    let id = environment.id();
    assert_eq!(id, "a6b5b2b967541b7a");
  }
}
