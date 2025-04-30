use std::fs;
use std::path::PathBuf;

use crate::env::Env;
use crate::platform::path_ext::find_ancestor_file;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Paths {
  /// $APVM_DIR (default $HOME/.local/.apvm)
  pub apvm_dir: PathBuf,
  /// $APVM_DIR/.temp
  pub temp: PathBuf,
  /// $APVM_DIR/global
  pub global: PathBuf,
  /// $APVM_DIR/versions
  pub versions: PathBuf,
  /// $APVM_DIR/versions/local
  pub versions_local: PathBuf,
  /// $APVM_DIR/versions/git
  pub versions_git: PathBuf,
  /// $APVM_DIR/versions/npm
  pub versions_npm: PathBuf,
  /// $PWD/node_modules/atlaspack/.version (recursive search)
  pub node_modules_atlaspack: Option<PathBuf>,
}

impl Paths {
  pub fn new(env: &Env) -> anyhow::Result<Self> {
    let apvm_dir = env.apvm_dir.clone();
    let apvm_global_dir = apvm_dir.join("global");
    let apvm_dir_temp = apvm_dir.join(".temp");
    let apvm_versions_dir = apvm_dir.join("versions");
    let apvm_versions_local_dir = apvm_versions_dir.join("local");
    let apvm_versions_git_dir = apvm_versions_dir.join("git");
    let apvm_versions_npm_dir = apvm_versions_dir.join("npm");
    let node_modules_atlaspack =
      find_ancestor_file(&env.pwd, PathBuf::from_iter(&["node_modules", "atlaspack"]))?
        .first()
        .cloned();

    if !fs::exists(&apvm_dir)? {
      fs::create_dir(&apvm_dir)?;
    }

    if !fs::exists(&apvm_versions_dir)? {
      fs::create_dir(&apvm_versions_dir)?;
    }

    if !fs::exists(&apvm_dir_temp)? {
      fs::create_dir(&apvm_dir_temp)?;
    }

    if !fs::exists(&apvm_versions_dir)? {
      fs::create_dir(&apvm_versions_dir)?;
    }

    if !fs::exists(&apvm_versions_local_dir)? {
      fs::create_dir(&apvm_versions_local_dir)?;
    }

    if !fs::exists(&apvm_versions_git_dir)? {
      fs::create_dir(&apvm_versions_git_dir)?;
    }

    if !fs::exists(&apvm_versions_npm_dir)? {
      fs::create_dir(&apvm_versions_npm_dir)?;
    }

    Ok(Self {
      apvm_dir,
      global: apvm_global_dir,
      temp: apvm_dir_temp,
      versions: apvm_versions_dir,
      versions_local: apvm_versions_local_dir,
      versions_git: apvm_versions_git_dir,
      versions_npm: apvm_versions_npm_dir,
      node_modules_atlaspack,
    })
  }
}
