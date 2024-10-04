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
use git2::{DiffOptions, Repository};
use serde::{Deserialize, Serialize};
use std::{
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
  process::Command,
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
  pub yarn_states: Vec<YarnSnapshot>,
}

impl VCSState {
  /// Read the VCS state from a repository root. Ignore dirty files matching
  /// the exclude patterns.
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
      &node_modules_path.parent().unwrap(),
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
