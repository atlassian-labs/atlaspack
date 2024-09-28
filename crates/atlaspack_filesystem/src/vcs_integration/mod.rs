use anyhow::anyhow;
use git2::{DiffOptions, Repository};
use serde::{Deserialize, Serialize};
use std::{
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
  process::Command,
};
use yarn_integration::{parse_yarn_lock, parse_yarn_state_file, YarnLock, YarnStateFile};

pub mod yarn_integration;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VCSState {
  pub git_hash: String,
  pub dirty_files: Vec<VCSFile>,
  pub yarn_states: Vec<YarnSnapshot>,
}

impl VCSState {
  pub fn read_from_repository(
    path: &Path,
    exclude_patterns: &[String],
    failure_mode: FailureMode,
  ) -> anyhow::Result<VCSState> {
    let repo = Repository::open(path)?;
    let head = repo.revparse_single("HEAD")?.peel_to_commit()?;
    let git_hash = head.id().to_string();
    let file_listing = vcs_list_dirty_files(path, exclude_patterns)?;
    let yarn_states = list_yarn_states(path, failure_mode)?;

    Ok(VCSState {
      git_hash,
      dirty_files: file_listing,
      yarn_states,
    })
  }
}

pub fn list_yarn_states(
  repo: &Path,
  failure_mode: FailureMode,
) -> anyhow::Result<Vec<YarnSnapshot>> {
  let files = vcs_list_files(repo)?;
  let yarn_lock_files = files
    .iter()
    .filter(|file| file.ends_with("yarn.lock"))
    .cloned()
    .collect::<Vec<_>>();

  let mut yarn_states = Vec::new();

  for file in yarn_lock_files {
    let yarn_lock_path = repo.join(file);
    let node_modules_relative_path = yarn_lock_path.parent().unwrap().join("node_modules");

    let yarn_lock_blob = std::fs::read(&yarn_lock_path)?;
    let yarn_lock: Result<YarnLock, _> = parse_yarn_lock(&String::from_utf8(yarn_lock_blob)?);
    if failure_mode != FailureMode::FailOnMissingNodeModules && yarn_lock.is_err() {
      continue;
    };
    let yarn_lock = yarn_lock?;

    let node_modules_path = repo.join(&node_modules_relative_path);
    let yarn_state = parse_yarn_state_file(&node_modules_path).map_err(|err| {
      tracing::warn!(
        "Failed to read .yarn-state.yml {node_modules_relative_path:?} {}",
        err
      );

      anyhow!("Failed to read .yarn-state.yml at {node_modules_relative_path:?}: {err}")
    });
    if failure_mode != FailureMode::FailOnMissingNodeModules && yarn_state.is_err() {
      continue;
    };
    let yarn_state = yarn_state?;
    let yarn_snapshot = YarnSnapshot {
      yarn_lock_path,
      yarn_lock,
      yarn_state,
    };
    yarn_states.push(yarn_snapshot);
  }

  Ok(yarn_states)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YarnSnapshot {
  pub yarn_lock_path: PathBuf,
  pub yarn_lock: YarnLock,
  pub yarn_state: YarnStateFile,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VCSFile {
  pub path: PathBuf,
  pub hash: u64,
}

fn get_file_contents_at_commit(
  repo: &Repository,
  commit: &git2::Commit,
  path: &Path,
) -> anyhow::Result<String> {
  let tree = commit.tree()?;
  let entry = tree.get_path(path)?;
  let blob = entry
    .to_object(&repo)?
    .into_blob()
    .map_err(|_| anyhow::anyhow!("Failed to read yarn.lock from git"))?;
  let contents = blob.content();
  Ok(String::from_utf8(contents.to_vec())?)
}

#[derive(Debug, PartialEq)]
pub enum FailureMode {
  IgnoreMissingNodeModules,
  FailOnMissingNodeModules,
}

pub fn get_changed_files(
  repo_path: &Path,
  old_rev: &str,
  new_rev: &str,
  failure_mode: FailureMode,
) -> anyhow::Result<Vec<PathBuf>> {
  let repo = Repository::open(repo_path)?;
  let old_commit = repo.revparse_single(&old_rev)?.peel_to_commit()?;
  let new_commit = repo.revparse_single(&new_rev)?.peel_to_commit()?;

  tracing::debug!("Calculating git diff");
  let mut diff_options = DiffOptions::new();
  let diff = repo.diff_tree_to_tree(
    Some(&old_commit.tree()?),
    Some(&new_commit.tree()?),
    Some(&mut diff_options),
  )?;

  let mut changed_files = Vec::new();
  diff.foreach(
    &mut |delta, _| {
      if let Some(path) = delta.new_file().path() {
        changed_files.push(repo_path.join(path));
      }
      true
    },
    None,
    None,
    None,
  )?;

  tracing::debug!("Reading yarn.lock from {} and {}", old_rev, new_rev);
  let yarn_lock_changes = changed_files
    .iter()
    .filter(|file| file.file_name().unwrap() == "yarn.lock")
    .cloned()
    .collect::<Vec<_>>();
  for yarn_lock_path in yarn_lock_changes {
    tracing::debug!("Found yarn.lock in changed files");
    let yarn_lock_path = yarn_lock_path.strip_prefix(repo_path)?;
    let node_modules_relative_path = yarn_lock_path.parent().unwrap().join("node_modules");

    let old_yarn_lock_blob = get_file_contents_at_commit(&repo, &old_commit, &yarn_lock_path)?;
    let old_yarn_lock: YarnLock = parse_yarn_lock(&old_yarn_lock_blob)?;
    let new_yarn_lock_blob = get_file_contents_at_commit(&repo, &new_commit, &yarn_lock_path)?;
    let new_yarn_lock: YarnLock = parse_yarn_lock(&new_yarn_lock_blob)?;

    let node_modules_path = repo_path.join(&node_modules_relative_path);
    let yarn_state = parse_yarn_state_file(&node_modules_path).map_err(|err| {
      tracing::warn!(
        "Failed to read .yarn-state.yml {node_modules_relative_path:?} {}",
        err
      );

      anyhow!("Failed to read .yarn-state.yml at {node_modules_relative_path:?}: {err}")
    });
    if failure_mode != FailureMode::FailOnMissingNodeModules && yarn_state.is_err() {
      continue;
    };
    let yarn_state = yarn_state?;

    tracing::debug!(
      "Reading node_modules state from {} and calculating diff",
      node_modules_path.display()
    );
    let node_modules_changes = yarn_integration::generate_events(
      &node_modules_path,
      &old_yarn_lock,
      &new_yarn_lock,
      &yarn_state,
    );
    changed_files.extend(node_modules_changes);
  }

  tracing::debug!("Done");

  Ok(changed_files)
}

pub fn vcs_list_files(dir: &Path) -> Result<Vec<String>, anyhow::Error> {
  let mut command = Command::new("git");
  command
    .arg("ls-files")
    .arg("-z") // We separate rows by \0 to avoid issues with newlines
    .current_dir(dir);
  let command = command.output()?;
  if !command.status.success() {
    tracing::error!("git lfs-files: {:?}", String::from_utf8(command.stderr));
    return Err(anyhow::anyhow!("Git ls-files failed"));
  }
  let output = String::from_utf8(command.stdout).unwrap();
  let lines = output.split_terminator('\0');

  let mut results = Vec::new();

  for path in lines {
    results.push(path.to_string());
  }

  Ok(results)
}

pub fn vcs_list_dirty_files(
  dir: &Path,
  exclude_patterns: &[String],
) -> Result<Vec<VCSFile>, anyhow::Error> {
  let mut command = Command::new("git");
  command
    .arg("ls-files")
    // .arg("--format=%(objectname)%(path)")
    .arg("--deleted")
    .arg("--modified")
    .arg("--others")
    .arg("-z") // We separate rows by \0 to avoid issues with newlines
    .current_dir(dir);
  for pattern in exclude_patterns {
    command.arg(format!("--exclude={}", pattern));
  }
  let command = command.output()?;
  if !command.status.success() {
    tracing::error!("git lfs-files: {:?}", String::from_utf8(command.stderr));
    return Err(anyhow::anyhow!("Git ls-files failed"));
  }
  let output = String::from_utf8(command.stdout).unwrap();
  let lines = output.split_terminator('\0');

  let mut results = Vec::new();

  for path in lines {
    let contents = std::fs::read(&dir.join(path))?;
    let mut state = std::collections::hash_map::DefaultHasher::new();
    contents.hash(&mut state);
    let hash = state.finish();
    results.push(VCSFile {
      path: path.into(),
      hash,
    });
  }

  Ok(results)
}
