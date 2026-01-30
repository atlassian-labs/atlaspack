use caniuse_serde::{
  AgentName, EmbeddedCanIUseDatabase, Feature as CanIUseFeature, FeatureName, SupportMaturity,
  Version,
};
use oxc_browserslist::{Distrib, Opts, execute};
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  sync::{Mutex, OnceLock},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserslistCacheEntry {
  pub(crate) entries: Vec<Distrib>,
  pub(crate) had_error: bool,
}

static BROWSERSLIST_CACHE: OnceLock<Mutex<HashMap<PathBuf, BrowserslistCacheEntry>>> =
  OnceLock::new();

pub(crate) fn browserslist_cache() -> &'static Mutex<HashMap<PathBuf, BrowserslistCacheEntry>> {
  BROWSERSLIST_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn cached_browserslist_entries(config_path: Option<&Path>) -> BrowserslistCacheEntry {
  let cache_key = config_path
    .map(Path::to_path_buf)
    .unwrap_or_else(|| PathBuf::from("."));

  if let Some(cached) = browserslist_cache()
    .lock()
    .expect("browserslist cache lock should not be poisoned")
    .get(&cache_key)
    .cloned()
  {
    return cached;
  }

  let computed = load_browserslist_entries(config_path);

  browserslist_cache()
    .lock()
    .expect("browserslist cache lock should not be poisoned")
    .insert(cache_key, computed.clone());

  computed
}

pub(crate) fn feature_supported_for_config(
  feature_name: FeatureName,
  config_path: Option<&Path>,
) -> (bool, Vec<String>) {
  let browserslist = cached_browserslist_entries(config_path);
  if browserslist.had_error {
    // On failure, stay permissive to avoid unexpectedly bloating output.
    return (true, vec!["<browserslist-error>".to_string()]);
  }

  let caniuse_feature = match feature_name.feature(&EmbeddedCanIUseDatabase) {
    Some(feature) => feature,
    // If the dataset is missing, do not block builds.
    None => return (true, vec!["<caniuse-feature-missing>".to_string()]),
  };

  let mut seen: Vec<String> = Vec::new();
  let mut all_supported = true;

  for entry in &browserslist.entries {
    seen.push(format_distrib(entry));

    let agent = map_agent_name(entry.name());

    if let Some(false) = supports_feature(&caniuse_feature, &agent, entry.version()) {
      all_supported = false;
    }
  }

  (all_supported, seen)
}

pub(crate) fn feature_supported_for_config_path(
  config_path: Option<PathBuf>,
  feature_name: FeatureName,
) -> (bool, Vec<String>) {
  feature_supported_for_config(feature_name, config_path.as_deref())
}

fn load_browserslist_entries(config_path: Option<&Path>) -> BrowserslistCacheEntry {
  let mut opts = Opts::default();
  opts.path = config_path.map(|path: &Path| path.to_string_lossy().into_owned());

  match execute(&opts) {
    Ok(entries) => BrowserslistCacheEntry {
      entries,
      had_error: false,
    },
    Err(_) => BrowserslistCacheEntry {
      entries: Vec::new(),
      had_error: true,
    },
  }
}

fn supports_feature(
  feature: &CanIUseFeature<'_>,
  agent: &AgentName,
  version: &str,
) -> Option<bool> {
  let version = Version::from(version);
  match feature.implementation(agent, &version) {
    Some(Some(support)) => Some(matches!(
      support.maturity(),
      SupportMaturity::SupportedByDefault
    )),
    // Unknown or missing data: remain permissive.
    _ => None,
  }
}

fn map_agent_name(name: &str) -> AgentName {
  match name {
    "ie" => AgentName::MicrosoftInternetExplorer,
    "edge" => AgentName::MicrosoftEdge,
    "firefox" => AgentName::MozillaFirefox,
    "chrome" => AgentName::GoogleChrome,
    "safari" => AgentName::AppleSafari,
    "opera" => AgentName::Opera,
    "ios_saf" => AgentName::AppleSafariIOs,
    "op_mini" => AgentName::OperaMini,
    "android" => AgentName::GoogleAndroidBrowserAndWebComponent,
    "bb" => AgentName::Blackberry,
    "op_mob" => AgentName::OperaMobile,
    "and_chr" => AgentName::GoogleChromeAndroid,
    "and_ff" => AgentName::MozillaFirefoxAndroid,
    "ie_mob" => AgentName::MicrosoftInternetExplorerMobile,
    "and_uc" => AgentName::UcBrowserAndroid,
    "samsung" => AgentName::SamsungBrowserAndroid,
    "and_qq" => AgentName::QqBrowserAndroid,
    other => AgentName::Unknown(other.to_string()),
  }
}

fn format_distrib(distrib: &Distrib) -> String {
  format!(
    "{}/{}",
    distrib.name().to_ascii_lowercase(),
    distrib.version().to_ascii_lowercase()
  )
}
