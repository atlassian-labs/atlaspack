use browserslist::Distrib;
use serde::Deserialize;
use serde::Serialize;

use super::browsers::Browsers;
use super::version::Version;
use super::OutputFormat;

/// The browsers list as it appears on the engines field.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EnginesBrowsers {
  List(Vec<String>),
  String(String),
}

impl Default for EnginesBrowsers {
  fn default() -> Self {
    Self::List(vec![
      String::from("last 2 versions"),
      String::from("> 0.25%"),
      String::from("not dead"),
    ])
  }
}

impl EnginesBrowsers {
  pub fn new(browser_list: Vec<String>) -> Self {
    Self::List(browser_list)
  }

  pub fn list(&self) -> Vec<String> {
    match self {
      Self::List(list) => list.clone(),
      Self::String(string) => vec![string.clone()],
    }
  }

  pub fn resolve(&self) -> Vec<Distrib> {
    let list = self.list();
    browserslist::resolve(list, &Default::default()).unwrap_or(Vec::new())
  }
}

impl From<EnginesBrowsers> for Browsers {
  fn from(engines_browsers: EnginesBrowsers) -> Self {
    let list = match engines_browsers {
      EnginesBrowsers::List(list) => list,
      EnginesBrowsers::String(string) => vec![string],
    };
    let distribs = browserslist::resolve(list, &Default::default()).unwrap_or(Vec::new());
    Browsers::from(distribs)
  }
}

/// The engines field in package.json
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Engines {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub atlaspack: Option<Version>,
  #[serde(default)]
  pub browsers: Option<EnginesBrowsers>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub electron: Option<Version>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub node: Option<Version>,
}

/// List of environment features that may be supported by an engine
#[derive(Debug)]
pub enum EnvironmentFeature {
  DynamicImport,
  WorkerModule,
}

impl From<EnvironmentFeature> for caniuse_database::BrowserFeature {
  fn from(feature: EnvironmentFeature) -> Self {
    match feature {
      EnvironmentFeature::DynamicImport => caniuse_database::BrowserFeature::Es6ModuleDynamicImport,
      EnvironmentFeature::WorkerModule => caniuse_database::BrowserFeature::Webworkers,
    }
  }
}

/// List of browsers to exclude when the esmodule target is specified based on
/// https://caniuse.com/#feat=es6-module
const _ESMODULE_BROWSERS: &[&str] = &[
  "not ie <= 11",
  "not edge < 16",
  "not firefox < 60",
  "not chrome < 61",
  "not safari < 11",
  "not opera < 48",
  "not ios_saf < 11",
  "not op_mini all",
  "not android < 76",
  "not blackberry > 0",
  "not op_mob > 0",
  "not and_chr < 76",
  "not and_ff < 68",
  "not ie_mob > 0",
  "not and_uc > 0",
  "not samsung < 8.2",
  "not and_qq > 0",
  "not baidu > 0",
  "not kaios > 0",
];

impl Engines {
  pub fn from_browserslist(browserslist: Vec<String>) -> Browsers {
    browserslist::resolve(browserslist, &Default::default())
      .map(|b| b.into())
      .unwrap_or_default()
  }

  // TODO Reinstate this so that engines.browsers are filtered out with ESMODULE_BROWSERS when
  // we are using an esmodule output format
  pub fn optimize(_engines: Engines, _output_format: OutputFormat) -> Engines {
    todo!()
  }

  #[tracing::instrument(level = "debug", skip(self))]
  pub fn supports(&self, feature: EnvironmentFeature) -> bool {
    let distribs = self
      .browsers
      .as_ref()
      .unwrap_or(&Default::default())
      .resolve();
    caniuse_database::check_browserslist_support(
      &caniuse_database::BrowserFeature::from(feature),
      &distribs,
    )
  }
}
