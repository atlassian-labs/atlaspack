//! This crate provides integration with `git` and `yarn.lock` such that
//! **atlaspack** can perform cache invalidation based on version-control
//! information, as opposed to filesystem events.
//!
//! There are a few motivations to do this:
//!
//! - It is significantly faster, in certain cases, to query `git` than it is to
//!   fetch large lists of events from the watcher
//! - This allows the current atlaspack caching system to work on CI
//!
//! # Implementation overview
//!
//! ## Yarn lock and yarn state
//!
//! `yarn` writes `yarn.lock` files, containing package resolutions. On `yarn v2`,
//! this file is YAML, and on `yarn v1` it is a custom format.
//!
//! The file contains mappings of dependency requirements to what package they've
//! resolved to. For example it might contain (some fields omitted):
//!
//! ```yaml
//! 'lodash@npm:^3':
//!   resolution: 'lodash@npm:3.10.1'
//!   checksum: 10c0/f5f6d3d87503c3f1db27d49b30a00bb38dc1bd9de716c5febe8970259cc7b447149a0e320452ccaf5996a7a4abd63d94df341bb91bd8d336584ad518d8eab144
//! ```
//!
//! Here `'lodash@npm:^3'` is the **requirement**, and `lodash@npm:3.10.1` is the
//! **resolution**.
//!
//! Furthermore, on the `node_modules/.yarn-state.yml` file, `yarn` stores all the
//! filepaths for each **resolution**. The `.yarn-state.yml` might look like:
//!
//! ```yaml
//! 'lodash@npm:3.10.1':
//!   locations:
//!     - 'node_modules/lodash'
//! ```
//!
//! ## Overview
//!
//! The overall idea would be to modify the `getEventsSince` and `writeSnapshot`
//! filesystem functions.
//!
//! The snapshot file will be modified to contain,
//! **in addition to the current watcher snapshot** some git/yarn related metadata.
//! The metadata stored will be:
//!
//! 1. the current git revision SHA hash
//! 2. a list of the dirty files and their content hashes
//! 3. if any yarn.lock file is dirty, its "yarn snapshot"
//!    - the yarn.lock file contents
//!    - the .yarn-state.yaml contents
//!    - the filepath
//!
//! When we switch branches, we will read this new snapshot, and query git for the
//! files that have changed between revisions. This list will not contain untracked
//! files, such as `node_modules`, hence we will integrate with `yarn`.
//!
//! If a `yarn.lock` file has changed between the revisions, we will parse its
//! state at the current revision and the snapshots revision. We will then diff the
//! `yarn.lock` files looking for changed **resolutions**.
//!
//! Once we have all changed resolutions, we will use the current `.yarn-state.yml`
//! file to expand them into file-paths. The old state file could be used to
//! this, because we do not necessarily need to mark removed dependency paths as
//! deleted.
//!
//! ### Untracked files
//!
//! In order to support cases where the server starts between two uncommitted
//! changesets, which would not be visible on the git diff, we will store the
//! content hashes of the uncommitted files. We will perform exclusion over this
//! list to only consider relevant files. This will also handle all other cases of
//! **untracked files** that git does not track, but that might be relevant to the
//! build, such as generated files. Even on large repositories, this has manageable
//! size, and it can be reduced by avoiding to have such code-gen assets outside of
//! VCS.
//!
//! In order to support cases where the user starts-up a build in a dirty repository
//! which has uncommitted changes to `yarn.lock` and the dependencies, we will store
//! the **contents of the yarn.lock and yarn-state.yml** files in the snapshot. This
//! can be done only when the `yarn.lock` file is dirty, since we can otherwise read
//! its contents from git; but we might want to always store the `yarn-state.yml`
//! file in order to support marking excluded dependencies as deleted.
//!
//! ### git integration
//!
//! The crate integrates with `libgit` and the `git` binary. Currently `libgit` is
//! linked dynamically with the binary, which means it must be present on the client
//! machine, but doesn't require us to bump `atlaspack` whenever security fixes are
//! done to `git`.
//!
//! Git is used to:
//!
//! - List files that are dirty/untracked/removed/modified in a repository
//! - Diff two revisions to find changed files between revisions
//! - Get the contents of the `yarn.lock` files at different revisions
//!
//! # Roll-out and validation
//!
//! We will validate that this implementation is correct by:
//!
//! - Integrating into a new `FileSystem` implementation under a feature-flag
//! - Initially we will simply write the snapshots but still return the native
//!   watcher events ; however, we will diff the two events lists and report
//!   mismatches
//! - Once there are no mismatches found in production roll-outs or our testing we
//!   will stop querying watcher for the initial events list when starting-up a
//!   development build
//! - We will then try to implement CI caching using this implementation
//! - To achieve that we will perform similar comparison to guarantee we are
//!   producing equivalent build outputs
//!

use anyhow::anyhow;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
  collections::{HashMap, HashSet},
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
  process::Command,
  time::Instant,
};
use yarn_integration::{parse_yarn_lock, parse_yarn_state_file, YarnLock, YarnStateFile};

pub mod yarn_integration;

/// A snapshot of the current VCS state of the repository.
///
/// This includes:
///
/// * Content hashes of dirty files
/// * The current git revision
/// * All yarn lockfile states found
///
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VCSState {
  pub git_hash: String,
  pub dirty_files: Vec<VCSFile>,
  pub dirty_files_execution_time: u32,
  pub yarn_states: Vec<YarnSnapshot>,
  pub yarn_states_execution_time: u32,
}

impl VCSState {
  pub fn from_git_hash(git_hash: String) -> Self {
    Self {
      git_hash,
      dirty_files: vec![],
      dirty_files_execution_time: 0,
      yarn_states: vec![],
      yarn_states_execution_time: 0,
    }
  }

  /// Read the VCS state from a repository root. Ignore dirty files matching
  /// the exclude patterns.
  pub fn read_from_repository(
    path: &Path,
    exclude_patterns: &[String],
    failure_mode: FailureMode,
  ) -> anyhow::Result<VCSState> {
    tracing::info!("Reading VCS state");
    let git_hash = rev_parse(path, "HEAD")?;
    tracing::info!("Found head commit");
    let files_listing_start_time = Instant::now();
    let file_listing = vcs_list_dirty_files(path, exclude_patterns)?;
    let files_listing_duration = files_listing_start_time
      .elapsed()
      .as_millis()
      .try_into()
      .unwrap_or(u32::MAX);
    tracing::info!("Listed dirty files in: {:?} ms", files_listing_duration);
    let yarn_states_start_time = Instant::now();
    let yarn_states = list_yarn_states(path, failure_mode)?;
    let yarn_states_duration = yarn_states_start_time
      .elapsed()
      .as_millis()
      .try_into()
      .unwrap_or(u32::MAX);
    tracing::info!("Listed yarn states in: {:?} ms", yarn_states_duration);

    Ok(VCSState {
      git_hash,
      dirty_files: file_listing,
      dirty_files_execution_time: files_listing_duration,
      yarn_states,
      yarn_states_execution_time: yarn_states_duration,
    })
  }
}

#[tracing::instrument(level = "info", skip_all)]
pub fn list_yarn_states(
  repo: &Path,
  failure_mode: FailureMode,
) -> anyhow::Result<Vec<YarnSnapshot>> {
  let yarn_lock_files = vcs_list_yarn_lock_files(repo)?;
  tracing::info!(?failure_mode, "Found yarn.lock files");

  let yarn_states = yarn_lock_files
    .par_iter()
    .map(|file| -> anyhow::Result<_> {
      let yarn_lock_path = repo.join(file);
      let node_modules_relative_path = yarn_lock_path.parent().unwrap().join("node_modules");

      let yarn_lock_blob = std::fs::read(&yarn_lock_path)
        .map_err(|err| anyhow!("Failed to read {yarn_lock_path:?} from FS: {err}"));

      if failure_mode != FailureMode::FailOnMissingNodeModules && yarn_lock_blob.is_err() {
        return Ok(None);
      };

      let yarn_lock_blob = yarn_lock_blob?;
      let yarn_lock: Result<YarnLock, _> = parse_yarn_lock(
        &String::from_utf8(yarn_lock_blob)
          .map_err(|err| anyhow!("Failed to parse {yarn_lock_path:?} as UTF-8: {err}"))?,
      )
      .map_err(|err| anyhow!("Failed to parse {yarn_lock_path:?}: {err}"));

      if failure_mode != FailureMode::FailOnMissingNodeModules && yarn_lock.is_err() {
        return Ok(None);
      };

      let yarn_lock = yarn_lock?;

      let node_modules_path = repo.join(&node_modules_relative_path);
      let yarn_state = parse_yarn_state_file(&node_modules_path).map_err(|err| {
        tracing::debug!(
          "Failed to read .yarn-state.yml {node_modules_relative_path:?} {}",
          err
        );

        anyhow!("Failed to read .yarn-state.yml at {node_modules_relative_path:?}: {err}")
      });
      if failure_mode != FailureMode::FailOnMissingNodeModules && yarn_state.is_err() {
        return Ok(None);
      };
      let yarn_state = yarn_state?;
      let yarn_snapshot = YarnSnapshot {
        yarn_lock_path: yarn_lock_path.strip_prefix(repo)?.to_path_buf(),
        yarn_lock,
        yarn_state,
      };

      Ok(Some(yarn_snapshot))
    })
    .filter_map(|result| match result {
      Ok(Some(yarn_snapshot)) => Some(Ok(yarn_snapshot)),
      Ok(None) => None,
      Err(err) => Some(Err(err)),
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

  Ok(yarn_states)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YarnSnapshot {
  pub yarn_lock_path: PathBuf,
  pub yarn_lock: YarnLock,
  pub yarn_state: YarnStateFile,
}

/// "Dirty" files are files modified in the current work-tree and uncommitted.
///
/// These files are hashed and stored in the snapshot.
///
/// Currently on boot all dirty files will be invalidated on the cache regardless of
/// whether they have changes or not.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VCSFile {
  pub path: PathBuf,
  /// Missing in case the file has been deleted
  pub hash: Option<String>,
}

fn get_file_contents_at_commit(
  repo: &Path,
  commit: &str,
  path: &Path,
) -> anyhow::Result<Option<String>> {
  let result = Command::new("git")
    .arg("cat-file")
    .arg("-p")
    .arg(format!("{}:{}", commit, path.display()))
    .current_dir(repo)
    .output();

  match result {
    Ok(contents) => {
      let stderr = String::from_utf8(contents.stderr).unwrap_or_default();
      if !contents.status.success() && stderr.contains("does not exist in") {
        return Ok(None);
      }

      if !contents.status.success() {
        let status_code = contents.status.code().unwrap_or(-1);
        return Err(anyhow::anyhow!(
          "Failed to read contents at {path:?} in revision {commit}: git failed with status {status_code}\n\nSTDERR: {stderr}"
        ));
      }

      let contents = String::from_utf8(contents.stdout)?;
      Ok(Some(contents))
    }
    Err(err) => Err(anyhow::anyhow!(
      "Failed to read contents at {path:?} in revision {commit}: {err}"
    )),
  }
}

#[derive(Debug, PartialEq)]
pub enum FailureMode {
  IgnoreMissingNodeModules,
  FailOnMissingNodeModules,
}

#[derive(Debug, PartialEq)]
pub struct FileChangeEvent {
  path: PathBuf,
  change_type: FileChangeType,
}

impl FileChangeEvent {
  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn change_type(&self) -> &FileChangeType {
    &self.change_type
  }

  pub fn change_type_str(&self) -> &str {
    match self.change_type() {
      FileChangeType::Create => "create",
      FileChangeType::Update => "update",
      FileChangeType::Delete => "delete",
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum FileChangeType {
  Create,
  Update,
  Delete,
}

pub fn get_changed_files_from_git(
  repo_path: &Path,
  old_commit: &str,
  new_commit: &str,
  dirty_files: &[VCSFile],
) -> anyhow::Result<Vec<FileChangeEvent>> {
  let mut tracked_changes = HashSet::new();
  let mut changed_files = Vec::new();

  // list current dirty files
  tracing::info!("Listing dirty files...");
  get_status_with_git_cli(repo_path, &mut tracked_changes, &mut changed_files)?;
  tracing::info!(num_dirty_files = changed_files.len(), "Listed dirty files");

  tracing::info!(?old_commit, ?new_commit, "Calculating git diff...");
  get_diff_with_git_cli(
    repo_path,
    old_commit,
    new_commit,
    &mut tracked_changes,
    &mut changed_files,
  )?;

  tracing::info!(
    num_changed_files = changed_files.len(),
    "Calculated git diff"
  );

  // we could content hash the files here to filter out changes that are not
  // relevant
  for dirty_file in dirty_files {
    if !tracked_changes.contains(&dirty_file.path) {
      let path = repo_path.join(dirty_file.path.clone());
      changed_files.push(FileChangeEvent {
        path,
        change_type: FileChangeType::Update,
      });
    }
  }

  Ok(changed_files)
}

fn get_diff_with_git_cli(
  repo_path: &Path,
  old_commit: &str,
  new_commit: &str,
  tracked_changes: &mut HashSet<PathBuf>,
  changed_files: &mut Vec<FileChangeEvent>,
) -> anyhow::Result<()> {
  if old_commit == new_commit {
    return Ok(());
  }

  let output = Command::new("git")
    .arg("diff")
    .arg("--name-status")
    .arg("--no-renames")
    // We need to list all changes even if `new_commit` is an ancestor of `old_commit`
    // https://git-scm.com/docs/git-diff
    // https://git-scm.com/docs/gitrevisions
    .arg(format!("{}..{}", old_commit, new_commit))
    .current_dir(repo_path)
    .output()?;

  if !output.status.success() {
    return Err(anyhow::anyhow!("Git diff failed"));
  }

  let output = String::from_utf8(output.stdout)?;
  let lines = output.split_terminator('\n');
  for line in lines {
    let status = line
      .chars()
      .next()
      .ok_or_else(|| anyhow!("Invalid git diff line: {}", line))?;
    let path = line.split_whitespace().skip(1).collect::<String>();
    let relative_path = PathBuf::from(path);
    let path = repo_path.join(&relative_path);
    let change_type = match status {
      'A' => FileChangeType::Create,
      'D' => FileChangeType::Delete,
      'M' => FileChangeType::Update,
      _ => FileChangeType::Update,
    };

    tracked_changes.insert(relative_path.clone());
    changed_files.push(FileChangeEvent { path, change_type });
  }

  Ok(())
}

/// Query git status from the CLI. This is because libgit2 does not support
/// sparse checkouts.
fn get_status_with_git_cli(
  repo_path: &Path,
  tracked_changes: &mut HashSet<PathBuf>,
  changed_files: &mut Vec<FileChangeEvent>,
) -> anyhow::Result<()> {
  let output = Command::new("git")
    .arg("status")
    .arg("--porcelain")
    .arg("--no-ignored")
    .current_dir(repo_path)
    .output()?;

  if !output.status.success() {
    return Err(anyhow::anyhow!("Git status failed"));
  }
  let output = String::from_utf8(output.stdout)?;
  let lines = output.split_terminator('\n');
  for line in lines {
    let status = line
      .chars()
      .nth(1)
      .ok_or_else(|| anyhow!("Invalid git status line: {}", line))?;
    let path = line.split_whitespace().skip(1).collect::<String>();
    let relative_path = PathBuf::from(path);
    let path = repo_path.join(&relative_path);
    let change_type = match status {
      'A' => FileChangeType::Create,
      'D' => FileChangeType::Delete,
      'M' => FileChangeType::Update,
      _ => continue,
    };
    tracked_changes.insert(relative_path.clone());
    changed_files.push(FileChangeEvent { path, change_type });
  }
  Ok(())
}

pub fn get_changed_files(
  repo_path: &Path,
  vcs_state: &VCSState,
  new_rev: Option<&str>,
  failure_mode: FailureMode,
) -> anyhow::Result<Vec<FileChangeEvent>> {
  let old_rev = &vcs_state.git_hash;
  let old_commit = rev_parse(repo_path, old_rev)?;
  let new_commit = rev_parse(repo_path, new_rev.unwrap_or("HEAD"))?;

  let mut changed_files =
    get_changed_files_from_git(repo_path, &old_commit, &new_commit, &vcs_state.dirty_files)?;
  tracing::trace!("Changed files: {:?}", changed_files);

  tracing::debug!("Reading yarn.lock from {} and {:?}", old_rev, new_rev);
  let yarn_lock_changes = changed_files
    .iter()
    .filter(|file| file.path.file_name().unwrap() == "yarn.lock")
    .map(|file| file.path.clone())
    .collect::<Vec<_>>();

  let yarn_snapshots_by_path = vcs_state
    .yarn_states
    .iter()
    .map(|yarn_snapshot| (yarn_snapshot.yarn_lock_path.clone(), yarn_snapshot))
    .collect::<HashMap<_, _>>();

  for yarn_lock_path in yarn_lock_changes {
    tracing::debug!(
      "Found yarn.lock in changed files: {}",
      yarn_lock_path.display()
    );
    let yarn_lock_path = yarn_lock_path.strip_prefix(repo_path)?;
    let node_modules_relative_path = yarn_lock_path.parent().unwrap().join("node_modules");

    tracing::debug!(
      "Reading yarn.lock ({}) from {:?} and {:?}",
      yarn_lock_path.display(),
      old_commit,
      new_commit
    );

    tracing::debug!("Querying yarn snapshots for {}", yarn_lock_path.display());
    let yarn_snapshot = yarn_snapshots_by_path.get(yarn_lock_path);
    let maybe_old_yarn_lock: Option<YarnLock> = if let Some(yarn_snapshot) = yarn_snapshot {
      // This handles the case where the yarn.lock was dirty in the build
      tracing::debug!("Using yarn snapshot for {}", yarn_lock_path.display());
      Some(yarn_snapshot.yarn_lock.clone())
    } else {
      tracing::debug!("Reading yarn.lock from git");
      let maybe_old_yarn_lock_blob =
        get_file_contents_at_commit(repo_path, &old_commit, yarn_lock_path)?;
      maybe_old_yarn_lock_blob
        .map(|s| parse_yarn_lock(&s))
        .transpose()?
    };

    let new_yarn_lock_blob: String = if new_rev.is_some() {
      get_file_contents_at_commit(repo_path, &new_commit, yarn_lock_path)?
        .ok_or_else(|| anyhow!("Expected lockfile to exist in current revision"))?
    } else {
      tracing::debug!("Reading raw yarn.lock on current file-system",);
      std::fs::read_to_string(repo_path.join(yarn_lock_path))
        .map_err(|err| anyhow!("Failed to read {yarn_lock_path:?} from file-system: {err}"))?
    };
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
      node_modules_path.parent().unwrap(),
      &maybe_old_yarn_lock,
      &new_yarn_lock,
      &yarn_state,
    );

    for change in node_modules_changes {
      changed_files.push(FileChangeEvent {
        path: change.clone(),
        change_type: FileChangeType::Delete,
      });
      changed_files.push(FileChangeEvent {
        path: change,
        change_type: FileChangeType::Create,
      });
    }
  }

  tracing::debug!("Done");

  Ok(changed_files)
}

pub fn rev_parse(dir: &Path, rev: &str) -> anyhow::Result<String> {
  let mut command = Command::new("git");
  command.arg("rev-parse").arg(rev).current_dir(dir);

  let command = command.output()?;
  if !command.status.success() {
    let stderr = String::from_utf8(command.stderr).unwrap_or_default();
    let exit_code = command.status.code().unwrap_or(-1);
    return Err(anyhow::anyhow!(
      "Git rev-parse failed (exit code {exit_code}): {stderr}"
    ));
  }

  Ok(String::from_utf8(command.stdout)?.trim().to_string())
}

pub fn vcs_list_yarn_lock_files(dir: &Path) -> Result<Vec<String>, anyhow::Error> {
  let mut command = Command::new("git");
  command
    .arg("ls-files")
    .arg("--cached")
    .arg("--exclude=yarn.lock")
    .arg("--ignored")
    .arg("-z") // We separate rows by \0 to avoid issues with newlines
    .current_dir(dir);
  let command = command.output()?;
  if !command.status.success() {
    tracing::error!("git ls-files: {:?}", String::from_utf8(command.stderr));
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

#[tracing::instrument(level = "info", skip_all)]
pub fn vcs_list_dirty_files(
  dir: &Path,
  exclude_patterns: &[String],
) -> Result<Vec<VCSFile>, anyhow::Error> {
  let mut command = Command::new("git");
  command
    .arg("ls-files")
    .arg("--deleted")
    .arg("--modified")
    .arg("--others")
    .arg("--exclude-standard")
    .arg("-z") // We separate rows by \0 to avoid issues with newlines
    .current_dir(dir);
  for pattern in exclude_patterns {
    command.arg(format!("--exclude={}", pattern));
  }
  let command = command.output()?;
  if !command.status.success() {
    let stderr =
      String::from_utf8(command.stderr).unwrap_or_else(|_| "non-utf8 error message".to_string());
    tracing::error!("git ls-files: {}", stderr);
    return Err(anyhow::anyhow!("Git ls-files failed:\n{}", stderr));
  }
  let output = String::from_utf8(command.stdout).unwrap();
  let lines: HashSet<_> = output.split_terminator('\0').map(String::from).collect();

  let results = lines
    .par_iter()
    .map(|relative_path| {
      tracing::info!("Hashing file {}", relative_path);
      let map_err = |err: std::io::Error| anyhow!("Failed to hash {relative_path:?}: {err}");

      let path = Path::new(relative_path);
      let path = dir.join(path);

      if !path.exists() {
        return Ok(VCSFile {
          path: relative_path.into(),
          hash: None,
        });
      }

      // We hash the contents of the file but if it's a symlink we hash the target
      // path instead rather than following the link.
      let metadata = std::fs::symlink_metadata(&path).map_err(map_err)?;
      let contents = if metadata.is_symlink() {
        std::fs::read_link(&path)
          .map_err(map_err)?
          .to_str()
          .unwrap()
          .as_bytes()
          .to_vec()
      } else {
        std::fs::read(&path).map_err(map_err)?
      };
      let mut state = std::collections::hash_map::DefaultHasher::new();
      contents.hash(&mut state);

      let hash = state.finish();
      let hash = format!("{:x}", hash);

      Ok(VCSFile {
        path: relative_path.into(),
        hash: Some(hash),
      })
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

  Ok(results)
}

#[cfg(test)]
mod test {
  use super::*;

  fn run_command(command: &mut Command) -> anyhow::Result<()> {
    let result = command.output()?;
    if !result.status.success() {
      return Err(anyhow::anyhow!(
        "Command failed: {}",
        String::from_utf8(result.stderr).unwrap()
      ));
    }
    Ok(())
  }

  fn create_test_repo(temp_dir: &tempfile::TempDir) -> anyhow::Result<PathBuf> {
    let repo_path = temp_dir.path().join("test-repo");
    std::fs::create_dir_all(&repo_path).unwrap();
    let mut command = Command::new("git");
    command.arg("init").current_dir(&repo_path);
    run_command(&mut command)?;

    let mut command = Command::new("git");
    command
      .arg("config")
      .arg("user.email")
      .arg("test-user@atlassian.com")
      .current_dir(&repo_path);
    run_command(&mut command)?;

    let mut command = Command::new("git");
    command
      .arg("config")
      .arg("user.name")
      .arg("test-user")
      .current_dir(&repo_path);
    run_command(&mut command)?;

    std::fs::write(repo_path.join("file.txt"), "initial contents")?;
    let mut command = Command::new("git");
    command.arg("add").arg(".").current_dir(&repo_path);
    run_command(&mut command)?;

    let mut command = Command::new("git");
    command
      .arg("commit")
      .arg("-m")
      .arg("Initial commit")
      .current_dir(&repo_path);
    run_command(&mut command)?;

    Ok(repo_path)
  }

  #[test]
  fn test_create_test_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir).unwrap();
    assert!(repo_path.exists());
  }

  #[test]
  fn test_rev_parse() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir).unwrap();
    let git_hash = rev_parse(&repo_path, "HEAD").unwrap();
    assert_ne!(git_hash, "HEAD");
  }

  #[test]
  fn test_get_file_contents_at_commit() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir).unwrap();
    let head_hash = rev_parse(&repo_path, "HEAD").unwrap();

    std::fs::write(repo_path.join("file.txt"), "new contents").unwrap();
    let mut command = Command::new("git");
    command.arg("add").arg(".").current_dir(&repo_path);
    run_command(&mut command).unwrap();

    let mut command = Command::new("git");
    command
      .arg("commit")
      .arg("-m")
      .arg("Update file")
      .current_dir(&repo_path);
    run_command(&mut command).unwrap();

    let current_hash = rev_parse(&repo_path, "HEAD").unwrap();

    let contents = get_file_contents_at_commit(&repo_path, &head_hash, Path::new("file.txt"))
      .unwrap()
      .unwrap();
    assert_eq!(contents, "initial contents".to_string());
    let contents = get_file_contents_at_commit(&repo_path, &current_hash, Path::new("file.txt"))
      .unwrap()
      .unwrap();
    assert_eq!(contents, "new contents".to_string());
  }

  #[test]
  fn test_get_contents_at_commit_missing_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir).unwrap();
    let head_hash = rev_parse(&repo_path, "HEAD").unwrap();

    let contents =
      get_file_contents_at_commit(&repo_path, &head_hash, Path::new("file1234.txt")).unwrap();
    assert!(contents.is_none());
  }

  #[test]
  fn test_get_changed_files_from_git() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir).unwrap();
    let head_hash = rev_parse(&repo_path, "HEAD").unwrap();

    let changes = get_changed_files_from_git(&repo_path, &head_hash, &head_hash, &[]).unwrap();
    assert_eq!(changes.len(), 0);

    std::fs::write(repo_path.join("file.txt"), "new contents").unwrap();
    let changes = get_changed_files_from_git(&repo_path, &head_hash, &head_hash, &[]).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path(), repo_path.join("file.txt"));
  }

  #[test]
  fn test_get_changed_files_from_git_on_rename() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = create_test_repo(&temp_dir).unwrap();
    let head_hash = rev_parse(&repo_path, "HEAD").unwrap();

    let changes = get_changed_files_from_git(&repo_path, &head_hash, &head_hash, &[]).unwrap();
    assert_eq!(changes.len(), 0);

    std::fs::rename(repo_path.join("file.txt"), repo_path.join("file2.txt")).unwrap();
    let mut command = Command::new("git");
    command.arg("add").arg(".").current_dir(&repo_path);
    run_command(&mut command).unwrap();
    let mut command = Command::new("git");
    command
      .arg("commit")
      .arg("-m")
      .arg("Rename file")
      .current_dir(&repo_path);
    run_command(&mut command).unwrap();

    let new_head_hash = rev_parse(&repo_path, "HEAD").unwrap();
    let changes = get_changed_files_from_git(&repo_path, &head_hash, &new_head_hash, &[]).unwrap();
    assert_eq!(
      changes,
      vec![
        FileChangeEvent {
          path: repo_path.join("file.txt"),
          change_type: FileChangeType::Delete,
        },
        FileChangeEvent {
          path: repo_path.join("file2.txt"),
          change_type: FileChangeType::Create,
        }
      ]
    );
  }
}
