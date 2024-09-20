//! Provides a small wrapper on top of caniuse data to check browser support for
//! a given browser feature.
//!
//! The data is taken from the `data.json` file from caniuse. A stripped down
//! version of file is vendored into the binary and is around 364KB.
//!
//! Alternatively the full database can be parsed and used to check support at
//! runtime.
//!
//! # Example
//!
//! ## Checking feature support for a specific version
//!
//! Check if Chrome 92 supports the WebUSB API.
//!
//! See [`check_browser_support`].
//!
//! ```rust
//! use caniuse_database::{BrowserFeature, BrowserAgent, Version, check_browser_support};
//!
//! let result: bool = check_browser_support(
//!    &BrowserFeature::Webusb,
//!    &BrowserAgent::Chrome,
//!    &Version::try_from("92.0.0").unwrap(),
//! );
//! assert!(result);
//! ````
//!
//! ## Checking feature support for a browserlists
//!
//! Use [`browserlist::resolve`] to check for feature support against a certain
//! browserlist query.
//!
//! See [`check_browserslist_support`].
//!
//! ```rust
//! use caniuse_database::{BrowserFeature, check_browserslist_support};
//!
//! let list = browserslist::resolve(&["last 2 chrome versions"], &Default::default()).unwrap();
//! let result: bool = check_browserslist_support(&BrowserFeature::ArrowFunctions, &list);
//! assert!(result);
//! ```
//!
//! # Simple version parsing
//!
//! We add a simplistic version parser to parse `1.2` or `1.0` or `1.2-1.3`
//! into [`Version`] and [`VersionRange`].
//!
//! # Caveats
//!
//! * Technology preview / non-numeric versions are not supported.
use std::collections::HashMap;

use lazy_static::lazy_static;

pub use generated::*;
pub use permissive_semver::*;
use serde::{Deserialize, Serialize};

mod generated;
mod permissive_semver;

static BROWSER_FEATURE_STATS_JSON: &str = include_str!("./data.json");

type BrowserFeatureKey = String;
type BrowserAgentKey = String;

/// The `data`/stats field in caniuse JSON data. This is generated with `generate.mjs`
type BrowserFeatureStats = HashMap<BrowserFeatureKey, HashMap<BrowserAgentKey, BrowserFeatureStat>>;

lazy_static! {
  static ref BROWSER_FEATURE_STATS: BrowserFeatureStats =
    serde_json::from_str(BROWSER_FEATURE_STATS_JSON).unwrap();
  static ref BROWSER_FEATURES: BrowsersFeaturesData =
    BrowsersFeaturesData::new(&BROWSER_FEATURE_STATS).unwrap();
}

type BrowserFeatureStat = HashMap<String, u8>;

#[derive(Debug, thiserror::Error)]
pub enum BrowserFeaturesError {
  #[error("Serde error: {0}")]
  SerdeError(#[from] serde_json::Error),
  #[error("IO error: {0}")]
  IOError(#[from] std::io::Error),
  #[error("Invalid version found at {browser_key} for {feature_key}: {version}")]
  InvalidVersion {
    browser_key: String,
    feature_key: String,
    version: String,
    parse_error: VersionParseError,
  },
}

/// Holds data to perform browser feature checks.
///
/// Use this if you want to load the browser feature data at runtime from a
/// JSON caniuse database.
pub struct BrowsersFeaturesData {
  data: HashMap<BrowserFeature, HashMap<BrowserAgent, Vec<VersionRange>>>,
}

impl BrowsersFeaturesData {
  /// Construct from data. This is the `.data` field in https://github.com/Fyrd/caniuse/blob/120453c0f736db1025cd3498328c2344d082fd52/data.json
  pub fn new(
    data: &HashMap<BrowserFeatureKey, HashMap<BrowserAgentKey, BrowserFeatureStat>>,
  ) -> Result<Self, BrowserFeaturesError> {
    // write this with a for loop
    let mut features = HashMap::new();
    for (feature_key, support_map) in data.iter() {
      let feature = BrowserFeature::from_key(&feature_key);
      let mut support = HashMap::new();
      for (browser_key, version_map) in support_map.iter() {
        let agent = BrowserAgent::from_key(&browser_key);
        let mut versions = Vec::new();
        for (version, support) in version_map.iter() {
          // Safari Technology Preview and Opera mini ALL versions are not supported.
          if version.contains("TP") || version == "all" {
            continue;
          }

          if *support != 0 {
            let range = VersionRange::parse(&version).map_err(|parse_error| {
              BrowserFeaturesError::InvalidVersion {
                browser_key: browser_key.clone(),
                feature_key: feature_key.clone(),
                version: version.clone(),
                parse_error,
              }
            })?;
            versions.push(range);
          }
        }
        support.insert(agent, versions);
      }
      features.insert(feature, support);
    }

    Ok(BrowsersFeaturesData { data: features })
  }

  pub fn check_browser_support(
    &self,
    feature: &BrowserFeature,
    agent: &BrowserAgent,
    version: &Version,
  ) -> bool {
    let Some(support) = self.data.get(feature) else {
      return false;
    };
    let Some(versions) = support.get(agent) else {
      return false;
    };

    versions.iter().any(|range| range.satisfies(&version))
  }

  /// Load from a path to a caniuse JSON database.
  ///
  /// This path should contain a file like https://github.com/Fyrd/caniuse/blob/120453c0f736db1025cd3498328c2344d082fd52/data.json
  ///
  /// ## Example
  /// ```
  /// use caniuse_database::{BrowserAgent, BrowserFeature, Version, BrowsersFeaturesData};
  ///
  /// let data = BrowsersFeaturesData::load_from_path("data.json").unwrap();
  /// let supports = data.check_browser_support(
  ///     &BrowserFeature::Webusb,
  ///     &BrowserAgent::Chrome,
  ///     &Version::try_from("92.0.0").unwrap()
  /// );
  /// assert!(supports);
  /// ```
  pub fn load_from_path(path: &str) -> Result<Self, BrowserFeaturesError> {
    let contents = std::fs::read_to_string(path)?;
    let data: CanIUseDatabase = serde_json::from_str(contents.as_str())?;

    Ok(Self::from(data))
  }
}

impl TryFrom<BrowserFeatureStats> for BrowsersFeaturesData {
  type Error = BrowserFeaturesError;

  fn try_from(value: BrowserFeatureStats) -> Result<Self, Self::Error> {
    BrowsersFeaturesData::new(&value)
  }
}

/// Check if a feature is supported on a certain browser version.
///
/// # Examples
/// ```rust
/// use caniuse_database::{BrowserFeature, BrowserAgent, Version, check_browser_support};
///
/// let result = check_browser_support(
///     &BrowserFeature::Webusb,
///     &BrowserAgent::Chrome,
///     &Version::try_from("92.0.0").unwrap(),
/// );
/// assert!(result);
/// ```
pub fn check_browser_support(
  feature: &BrowserFeature,
  agent: &BrowserAgent,
  version: &Version,
) -> bool {
  BROWSER_FEATURES.check_browser_support(feature, agent, version)
}

/// Check if a feature is supported on a certain [`browserslist`] query result.
///
/// # Examples
/// ```rust
/// use caniuse_database::{BrowserFeature, check_browserslist_support};
///
/// let list = browserslist::resolve(&["last 2 chrome versions"], &Default::default()).unwrap();
///
/// let result = check_browserslist_support(&BrowserFeature::ArrowFunctions, &list);
/// assert!(result);
///
/// let list = browserslist::resolve(&["op_mini all"], &Default::default()).unwrap();
/// let result = check_browserslist_support(&BrowserFeature::ArrowFunctions, &list);
/// assert!(!result);
///
/// let list = browserslist::resolve(&["> 0.5%"], &Default::default()).unwrap();
/// let result = check_browserslist_support(&BrowserFeature::ArrowFunctions, &list);
/// assert!(!result);
/// ```
pub fn check_browserslist_support(
  feature: &BrowserFeature,
  list: &[browserslist::Distrib],
) -> bool {
  println!("list {:?}", list);
  for distrib in list {
    let browser = BrowserAgent::from_key(distrib.name());
    let Ok(version) = Version::try_from(distrib.version()) else {
      return false;
    };
    println!("Checking support for {:?} {:?}", browser, version);
    if !check_browser_support(feature, &browser, &version) {
      return false;
    }
  }
  true
}

/// In case the can-i-use database needs to be parsed use this type then
/// convert into [`BrowsersFeaturesData`].
#[derive(Deserialize, Serialize)]
pub struct CanIUseDatabase {
  pub data: HashMap<BrowserFeatureKey, CanIUseFeatureData>,
}

#[derive(Deserialize, Serialize)]
pub struct CanIUseFeatureData {
  pub stats: HashMap<BrowserAgentKey, HashMap<String, String>>,
}

impl From<CanIUseDatabase> for BrowsersFeaturesData {
  fn from(value: CanIUseDatabase) -> Self {
    let mut features = HashMap::new();
    for (feature_key, feature_data) in value.data.iter() {
      let mut support = HashMap::new();
      let feature = BrowserFeature::from_key(&feature_key);
      for (browser_key, version_map) in feature_data.stats.iter() {
        let mut versions = Vec::new();
        let agent = BrowserAgent::from_key(&browser_key);

        for (version, support) in version_map.iter() {
          // Safari Technology Preview and Opera mini ALL versions are not supported.
          if version.contains("TP") || version == "all" {
            continue;
          }

          if *support == "y" {
            let range = VersionRange::parse(&version).unwrap();
            versions.push(range);
          }
        }

        support.insert(agent, versions);
      }

      features.insert(feature, support);
    }
    Self { data: features }
  }
}
