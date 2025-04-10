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
    let repo = Repository::open(path)?;
    let head = repo.revparse_single("HEAD")?.peel_to_commit()?;
    let git_hash = head.id().to_string();
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
  tracing::info!("Found yarn.lock files");

  let yarn_states = yarn_lock_files
    .par_iter()
    .map(|file| -> anyhow::Result<_> {
      let yarn_lock_path = repo.join(file);
      let node_modules_relative_path = yarn_lock_path.parent().unwrap().join("node_modules");

      let yarn_lock_blob = std::fs::read(&yarn_lock_path)?;
      let yarn_lock: Result<YarnLock, _> = parse_yarn_lock(&String::from_utf8(yarn_lock_blob)?);
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VCSFile {
  pub path: PathBuf,
  pub hash: String,
}

fn get_file_contents_at_commit(
  repo: &Repository,
  commit: &git2::Commit,
  path: &Path,
) -> anyhow::Result<Option<String>> {
  let tree = commit.tree()?;
  if let Ok(entry) = tree.get_path(path) {
    let blob = entry
      .to_object(repo)?
      .into_blob()
      .map_err(|_| anyhow::anyhow!("Failed to read yarn.lock from git"))?;
    let contents = blob.content();
    Ok(Some(String::from_utf8(contents.to_vec())?))
  } else {
    Ok(None)
  }
}

#[derive(Debug, PartialEq)]
pub enum FailureMode {
  IgnoreMissingNodeModules,
  FailOnMissingNodeModules,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum FileChangeType {
  Create,
  Update,
  Delete,
}

pub fn get_changed_files_from_git(
  repo_path: &Path,
  repo: &Repository,
  old_commit: &git2::Commit<'_>,
  new_commit: &git2::Commit<'_>,
  dirty_files: &[VCSFile],
) -> anyhow::Result<Vec<FileChangeEvent>> {
  let mut tracked_changes = HashSet::new();
  let mut changed_files = Vec::new();

  // list current dirty files
  tracing::info!("Listing dirty files");

  get_status_with_git_cli(repo_path, &mut tracked_changes, &mut changed_files)?;

  tracing::info!("Calculating git diff");
  let mut diff_options = DiffOptions::new();

  let diff = repo.diff_tree_to_tree(
    Some(&old_commit.tree()?),
    Some(&new_commit.tree()?),
    Some(&mut diff_options),
  )?;

  diff.foreach(
    &mut |delta, _| {
      if let Some(new_file_path) = delta.new_file().path() {
        let new_file_path = repo_path.join(new_file_path);

        let status = delta.status();
        if status == git2::Delta::Renamed {
          if let Some(old_file_path) = delta.old_file().path() {
            let old_file_path = repo_path.join(old_file_path);
            tracked_changes.insert(old_file_path.clone());
            changed_files.push(FileChangeEvent {
              path: old_file_path,
              change_type: FileChangeType::Delete,
            });
          }

          tracked_changes.insert(new_file_path.clone());
          changed_files.push(FileChangeEvent {
            path: new_file_path,
            change_type: FileChangeType::Create,
          });
          return true;
        }

        changed_files.push(FileChangeEvent {
          path: new_file_path,
          change_type: match status {
            git2::Delta::Added => FileChangeType::Create,
            git2::Delta::Modified => FileChangeType::Update,
            git2::Delta::Deleted => FileChangeType::Delete,
            git2::Delta::Unmodified => FileChangeType::Update,
            git2::Delta::Copied => FileChangeType::Create,
            git2::Delta::Ignored => FileChangeType::Update,
            git2::Delta::Untracked => FileChangeType::Update,
            git2::Delta::Typechange => FileChangeType::Update,
            git2::Delta::Unreadable => FileChangeType::Update,
            git2::Delta::Conflicted => FileChangeType::Update,
            git2::Delta::Renamed => panic!("Impossible branch"),
          },
        });
      }
      true
    },
    None,
    None,
    None,
  )?;

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
    let path = line.chars().skip(3).collect::<String>();
    let path = repo_path.join(path);
    let change_type = match status {
      'A' => FileChangeType::Create,
      'D' => FileChangeType::Delete,
      'M' => FileChangeType::Update,
      _ => continue,
    };
    tracked_changes.insert(path.clone());
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
  let repo = Repository::open(repo_path)?;
  let old_rev = &vcs_state.git_hash;
  let old_commit = repo.revparse_single(old_rev)?.peel_to_commit()?;
  let new_commit = repo
    .revparse_single(new_rev.unwrap_or("HEAD"))?
    .peel_to_commit()?;

  let mut changed_files = get_changed_files_from_git(
    repo_path,
    &repo,
    &old_commit,
    &new_commit,
    &vcs_state.dirty_files,
  )?;
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
        get_file_contents_at_commit(&repo, &old_commit, yarn_lock_path)?;
      maybe_old_yarn_lock_blob
        .map(|s| parse_yarn_lock(&s))
        .transpose()?
    };

    let new_yarn_lock_blob: String = if new_rev.is_some() {
      get_file_contents_at_commit(&repo, &new_commit, yarn_lock_path)?
        .ok_or_else(|| anyhow!("Expected lockfile to exist in current revision"))?
    } else {
      tracing::debug!("Reading raw yarn.lock on current file-system",);
      std::fs::read_to_string(repo_path.join(yarn_lock_path))?
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
  let lines: Vec<_> = output.split_terminator('\0').map(String::from).collect();

  let results = lines
    .par_iter()
    .map(|relative_path| {
      tracing::info!("Hashing file {}", relative_path);
      let path = Path::new(relative_path);
      let path = dir.join(path);

      // We hash the contents of the file but if it's a symlink we hash the target
      // path instead rather than following the link.
      let metadata = std::fs::symlink_metadata(&path)?;
      let contents = if metadata.is_symlink() {
        std::fs::read_link(&path)?
          .to_str()
          .unwrap()
          .as_bytes()
          .to_vec()
      } else {
        std::fs::read(&path)?
      };
      let mut state = std::collections::hash_map::DefaultHasher::new();
      contents.hash(&mut state);

      let hash = state.finish();
      let hash = format!("{:x}", hash);

      Ok(VCSFile {
        path: relative_path.into(),
        hash,
      })
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

  Ok(results)
}
