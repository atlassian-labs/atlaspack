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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BrowserslistCacheKey {
  pub(crate) path: PathBuf,
  pub(crate) env: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserslistCacheEntry {
  pub(crate) entries: Vec<Distrib>,
  pub(crate) had_error: bool,
}

static BROWSERSLIST_CACHE: OnceLock<Mutex<HashMap<BrowserslistCacheKey, BrowserslistCacheEntry>>> =
  OnceLock::new();

/// Returns the global cache for browserslist entries.
/// Public for test usage.
pub fn browserslist_cache() -> &'static Mutex<HashMap<BrowserslistCacheKey, BrowserslistCacheEntry>>
{
  BROWSERSLIST_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn cached_browserslist_entries(
  config_path: Option<&Path>,
  env: Option<&str>,
) -> BrowserslistCacheEntry {
  let cache_key = BrowserslistCacheKey {
    path: config_path
      .map(Path::to_path_buf)
      .unwrap_or_else(|| PathBuf::from(".")),
    env: env.map(String::from),
  };

  if let Some(cached) = browserslist_cache()
    .lock()
    .expect("browserslist cache lock should not be poisoned")
    .get(&cache_key)
    .cloned()
  {
    return cached;
  }

  let computed = load_browserslist_entries(config_path, env);

  browserslist_cache()
    .lock()
    .expect("browserslist cache lock should not be poisoned")
    .insert(cache_key, computed.clone());

  computed
}

pub(crate) fn feature_supported_for_config(
  feature_name: FeatureName,
  config_path: Option<&Path>,
  env: Option<&str>,
) -> (bool, Vec<String>) {
  let trace = std::env::var("COMPILED_CSS_TRACE").is_ok();
  let browserslist = cached_browserslist_entries(config_path, env);

  if trace {
    eprintln!(
      "[browserslist] feature_supported_for_config path={:?} env={:?} had_error={} entry_count={}",
      config_path,
      env,
      browserslist.had_error,
      browserslist.entries.len()
    );
  }

  if browserslist.had_error {
    // Match caniuse-api: if browserslist resolution fails, treat as unsupported.
    if trace {
      eprintln!("[browserslist] returning initial_support=false (had_error)");
    }
    return (false, vec!["<browserslist-error>".to_string()]);
  }

  let caniuse_feature = match feature_name.feature(&EmbeddedCanIUseDatabase) {
    Some(feature) => feature,
    // If the dataset is missing, do not block builds.
    None => {
      if trace {
        eprintln!("[browserslist] caniuse feature missing, returning initial_support=false");
      }
      return (false, vec!["<caniuse-feature-missing>".to_string()]);
    }
  };

  let mut seen: Vec<String> = Vec::new();
  let mut all_supported = true;
  let mut unsupported: Vec<String> = Vec::new();

  if browserslist.entries.is_empty() {
    if trace {
      eprintln!("[browserslist] no resolved browsers, returning initial_support=false");
    }
    return (false, vec![]);
  }

  for entry in &browserslist.entries {
    let dist = format_distrib(entry);
    seen.push(dist.clone());

    let agent = map_agent_name(entry.name());

    match supports_feature(&caniuse_feature, &agent, entry.version()) {
      Some(true) => {}
      Some(false) => {
        all_supported = false;
        unsupported.push(dist);
      }
      None => {
        // Unknown versions are ignored to match caniuse-api/isSupported behavior,
        // which runs browserslist with ignoreUnknownVersions=true.
      }
    }
  }

  if trace {
    eprintln!(
      "[browserslist] css-initial-value all_supported={} unsupported_count={}",
      all_supported,
      unsupported.len()
    );
    if !unsupported.is_empty() {
      eprintln!(
        "[browserslist] unsupported browsers (first 20): {:?}",
        &unsupported[..unsupported.len().min(20)]
      );
    }
    if seen.len() <= 30 {
      eprintln!("[browserslist] resolved browsers: {:?}", seen);
    } else {
      eprintln!(
        "[browserslist] resolved browsers (first 15): {:?} ... (total {})",
        &seen[..15],
        seen.len()
      );
    }
  }

  (all_supported, seen)
}

pub(crate) fn feature_supported_for_config_path(
  config_path: Option<PathBuf>,
  feature_name: FeatureName,
  env: Option<&str>,
) -> (bool, Vec<String>) {
  feature_supported_for_config(feature_name, config_path.as_deref(), env)
}

fn load_browserslist_entries(
  config_path: Option<&Path>,
  env: Option<&str>,
) -> BrowserslistCacheEntry {
  let trace = std::env::var("COMPILED_CSS_TRACE").is_ok();
  let mut opts = Opts::default();
  opts.path = config_path.map(|path: &Path| path.to_string_lossy().into_owned());
  opts.env = env.map(String::from);

  let result = match execute(&opts) {
    Ok(entries) => {
      if trace {
        eprintln!(
          "[browserslist] load_browserslist_entries path={:?} env={:?} ok count={}",
          config_path,
          env,
          entries.len()
        );
      }
      BrowserslistCacheEntry {
        entries,
        had_error: false,
      }
    }
    Err(e) => {
      if trace {
        eprintln!(
          "[browserslist] load_browserslist_entries path={:?} env={:?} error={:?}",
          config_path, env, e
        );
      }
      BrowserslistCacheEntry {
        entries: Vec::new(),
        had_error: true,
      }
    }
  };
  result
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
      // Align with caniuse-api's isSupported: only treat "y" as supported.
      SupportMaturity::SupportedByDefault
    )),
    // Unknown or missing data: caller may ignore for parity with caniuse-api.
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

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use std::fs;

  #[test]
  fn feature_supported_for_config_is_strict_like_caniuse_api() {
    let tmp = tempfile::tempdir().expect("tmpdir");
    fs::write(tmp.path().join(".browserslistrc"), "IE 11\n").expect("browserslist config write");

    browserslist_cache().lock().unwrap().clear();

    let (supported, _browsers) = feature_supported_for_config(
      FeatureName::from("css-rrggbbaa"),
      Some(tmp.path()),
      Some("production"),
    );

    // IE 11 does not support 4/8-digit hex; should be false.
    assert_eq!(supported, false);

    browserslist_cache()
      .lock()
      .unwrap()
      .remove(&BrowserslistCacheKey {
        path: tmp.path().to_path_buf(),
        env: Some("production".to_string()),
      });
  }

  #[test]
  fn feature_supported_for_config_ignores_unknown_versions() {
    let tmp = tempfile::tempdir().expect("tmpdir");
    fs::write(tmp.path().join(".browserslistrc"), "Chrome 143\n")
      .expect("browserslist config write");

    browserslist_cache().lock().unwrap().clear();

    let entries = cached_browserslist_entries(Some(tmp.path()), Some("production"));
    assert!(
      !entries.had_error,
      "browserslist should resolve entries for an explicit version"
    );
    assert!(
      !entries.entries.is_empty(),
      "browserslist should return at least one entry"
    );
    let feature_name = FeatureName::from("css-initial-value");
    let feature = feature_name
      .feature(&EmbeddedCanIUseDatabase)
      .expect("feature should exist");
    let has_unknown = entries.entries.iter().any(|entry| {
      supports_feature(&feature, &map_agent_name(entry.name()), entry.version()).is_none()
    });
    assert!(
      has_unknown,
      "expected at least one unknown browser version in caniuse data"
    );

    let (supported, _browsers) = feature_supported_for_config(
      FeatureName::from("css-initial-value"),
      Some(tmp.path()),
      Some("production"),
    );
    assert_eq!(supported, true);

    browserslist_cache()
      .lock()
      .unwrap()
      .remove(&BrowserslistCacheKey {
        path: tmp.path().to_path_buf(),
        env: Some("production".to_string()),
      });
  }
}
