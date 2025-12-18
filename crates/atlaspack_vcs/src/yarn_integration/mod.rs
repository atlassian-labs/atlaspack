//! Parse yarn lock files and state files to generate events for dependency
//! directories that change resolutions.
//!
//! This is intended to avoid relying on file-system events for untracked files
//! that are relevant for a bundler implementation. Since we'll simply read the
//! yarn state, we can generate events when the yarn.lock changes, but otherwise
//! rely on VCS as the source of truth for whether files have changed.
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::anyhow;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum YarnLockEntry {
  Resolution(YarnResolution),
  #[allow(unused)]
  Other(serde_yaml::Value),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct YarnResolution {
  resolution: String,
  checksum: Option<String>,
}

impl YarnResolution {
  pub fn resolution(&self) -> &str {
    &self.resolution
  }

  pub fn checksum(&self) -> Option<&str> {
    self.checksum.as_deref()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct YarnLock {
  /// Map of 'requirement' to 'resolution'
  ///
  /// * requirement -> `@atlaspack/core@^1.0.0`
  /// * resolution -> `@atlaspack/core@^1.0.0: @atlaspack/core@1.1.15`
  inner: HashMap<String, YarnLockEntry>,
}

impl YarnLock {
  pub fn inner(&self) -> &HashMap<String, YarnLockEntry> {
    &self.inner
  }
}

pub fn parse_yarn_lock(contents: &str) -> anyhow::Result<YarnLock> {
  let yarn_lock: YarnLock = serde_yaml::from_str(contents)?;
  yarn_lock.validate()?;
  Ok(yarn_lock)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YarnDependencyState {
  locations: Vec<String>,
}

impl YarnDependencyState {
  pub fn locations(&self) -> &[String] {
    &self.locations
  }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum YarnStateFileEntry {
  Dependency(YarnDependencyState),
  #[allow(unused)]
  Other(serde_yaml::Value),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct YarnStateFile {
  inner: HashMap<String, YarnStateFileEntry>,
}

impl YarnStateFile {
  pub fn validate(&self) -> anyhow::Result<()> {
    for (key, value) in &self.inner {
      if matches!(value, YarnStateFileEntry::Other(_)) && key != "__metadata" {
        return Err(anyhow::anyhow!("Invalid yarn-state.yml entry: {}", key));
      }
    }
    Ok(())
  }

  pub fn get(&self, key: &str) -> Option<&YarnDependencyState> {
    match self.inner.get(key) {
      Some(YarnStateFileEntry::Dependency(dependency_state)) => Some(dependency_state),
      _ => None,
    }
  }
}

impl YarnLock {
  pub fn validate(&self) -> anyhow::Result<()> {
    for (key, value) in &self.inner {
      if matches!(value, YarnLockEntry::Other(_)) && key != "__metadata" {
        return Err(anyhow::anyhow!("Invalid yarn.lock entry: {}", key));
      }
    }
    Ok(())
  }

  pub fn get(&self, key: &str) -> Option<&YarnResolution> {
    match self.inner.get(key) {
      Some(YarnLockEntry::Resolution(resolution)) => Some(resolution),
      _ => None,
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = (&String, &YarnResolution)> {
    self.inner.iter().filter_map(|(key, value)| match value {
      YarnLockEntry::Resolution(resolution) => Some((key, resolution)),
      _ => None,
    })
  }
}

pub fn parse_yarn_state_file(node_modules_directory: &Path) -> anyhow::Result<YarnStateFile> {
  let state_file_string = std::fs::read_to_string(node_modules_directory.join(".yarn-state.yml"))
    .map_err(|err| anyhow!("Failed to read yarn state file: {err:?}"))?;

  let state: YarnStateFile = serde_yaml::from_str(&state_file_string)?;
  state.validate()?;
  Ok(state)
}

fn get_changed_node_modules(
  node_modules_parent_path: &Path,
  old_yarn_lock: &Option<YarnLock>,
  new_yarn_lock: &YarnLock,
  state: &YarnStateFile,
) -> Vec<PathBuf> {
  let changed_resolutions = new_yarn_lock
    .iter()
    .filter_map(|(package_name, new_resolution)| {
      tracing::debug!("Checking resolution: {:?}", new_resolution.resolution);
      let Some(old_yarn_lock) = old_yarn_lock.as_ref() else {
        tracing::debug!(
          "No old resolution found for: {:?}",
          new_resolution.resolution
        );
        return Some(new_resolution);
      };
      let Some(old_resolution) = old_yarn_lock.get(package_name) else {
        tracing::debug!(
          "No old resolution found for: {:?}",
          new_resolution.resolution
        );
        return Some(new_resolution);
      };

      if old_resolution.resolution != new_resolution.resolution {
        tracing::debug!("Resolution changed: {:?}", new_resolution.resolution);
        Some(new_resolution)
      } else {
        None
      }
    });

  let mut changed_node_modules = vec![];
  for resolution in changed_resolutions {
    tracing::debug!("Changed resolution: {:?}", resolution.resolution);

    if let Some(dependency_state) = state.get(&resolution.resolution) {
      for location in &dependency_state.locations {
        if location.is_empty() {
          // the workspace is listed as a location, which we can skip
          continue;
        }
        let path = node_modules_parent_path.join(location);
        changed_node_modules.push(path);
      }
    }
  }

  changed_node_modules
}

fn expand_paths(directories: &Vec<PathBuf>) -> Vec<PathBuf> {
  directories
    .par_iter()
    .flat_map(|path| {
      jwalk::WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| !entry.path().is_dir())
        .map(|entry| entry.path().to_path_buf())
        .collect::<Vec<_>>()
    })
    .collect()
}

pub fn generate_events(
  node_modules_parent_path: &Path,
  old_yarn_lock: &Option<YarnLock>,
  new_yarn_lock: &YarnLock,
  state: &YarnStateFile,
) -> Vec<PathBuf> {
  let changed_node_modules = get_changed_node_modules(
    node_modules_parent_path,
    old_yarn_lock,
    new_yarn_lock,
    state,
  );

  expand_paths(&changed_node_modules)
}

#[cfg(test)]
mod test {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn test_generate_events() -> anyhow::Result<()> {
    let node_modules_parent_path = PathBuf::from("");
    let cargo_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let samples_path = cargo_path.join("samples");
    let old_yarn_lock_path = samples_path.join("old/yarn-lock");
    let new_yarn_lock_path = samples_path.join("new/yarn-lock");
    let yarn_state_path = samples_path.join("new/yarn-state.yml");

    let old_yarn_lock = parse_yarn_lock(&std::fs::read_to_string(old_yarn_lock_path)?)?;
    let new_yarn_lock = parse_yarn_lock(&std::fs::read_to_string(new_yarn_lock_path)?)?;
    let yarn_state: YarnStateFile =
      serde_yaml::from_str(&std::fs::read_to_string(yarn_state_path)?)?;
    yarn_state.validate()?;

    let events = get_changed_node_modules(
      &node_modules_parent_path,
      &Some(old_yarn_lock),
      &new_yarn_lock,
      &yarn_state,
    );
    let events = events
      .iter()
      .map(|path| path.to_str().unwrap())
      .collect::<Vec<_>>();

    assert_eq!(events, vec!["node_modules/lodash"]);

    Ok(())
  }

  #[test]
  fn test_generate_events_when_old_file_is_missing() -> anyhow::Result<()> {
    let node_modules_parent_path = PathBuf::from("");
    let cargo_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let samples_path = cargo_path.join("samples");
    let new_yarn_lock_path = samples_path.join("new/yarn-lock");
    let yarn_state_path = samples_path.join("new/yarn-state.yml");

    let new_yarn_lock = parse_yarn_lock(&std::fs::read_to_string(new_yarn_lock_path)?)?;
    let yarn_state: YarnStateFile =
      serde_yaml::from_str(&std::fs::read_to_string(yarn_state_path)?)?;
    yarn_state.validate()?;

    let events = get_changed_node_modules(
      &node_modules_parent_path,
      &None,
      &new_yarn_lock,
      &yarn_state,
    );
    let events = events
      .iter()
      .map(|path| path.to_str().unwrap())
      .collect::<Vec<_>>();

    assert_eq!(events, vec!["node_modules/lodash"]);

    Ok(())
  }

  #[test]
  fn test_expand_paths() {
    // create temporary directory with files
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = temp_dir.path();
    let file1 = temp_dir_path.join("file1");
    let file2 = temp_dir_path.join("file2");
    // create sub-directory
    let sub_dir = temp_dir_path.join("sub");
    std::fs::create_dir(&sub_dir).unwrap();
    let file3 = sub_dir.join("file3");
    let file4 = sub_dir.join("file4");

    std::fs::write(&file1, "file1").unwrap();
    std::fs::write(&file2, "file2").unwrap();
    std::fs::write(&file3, "file3").unwrap();
    std::fs::write(&file4, "file4").unwrap();

    let files = expand_paths(&vec![temp_dir_path.to_path_buf()]);
    let mut files = files
      .iter()
      .map(|path| path.to_str().unwrap())
      .collect::<Vec<_>>();
    files.sort();

    assert_eq!(
      &files,
      &[
        file1.to_str().unwrap(),
        file2.to_str().unwrap(),
        file3.to_str().unwrap(),
        file4.to_str().unwrap()
      ]
    );
  }
}
