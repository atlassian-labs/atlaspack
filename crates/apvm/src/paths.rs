use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::env::Env;
use crate::platform::path_ext::*;
use crate::platform::temp_dir::TempDir;

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub struct Paths {
  /// $APVM_DIR (default $HOME/.local/.apvm)
  pub apvm_dir: PathBuf,
  /// $APVM_DIR/.temp
  pub temp: PathBuf,
  /// $APVM_DIR/global
  pub global: PathBuf,
  /// $APVM_DIR/global/atlaspack
  pub global_atlaspack: PathBuf,
  /// $APVM_DIR/global/version
  pub global_version: PathBuf,
  /// $APVM_DIR/versions_v1
  pub versions_v1: PathBuf,
  /// $PWD/node_modules (recursive search)
  pub node_modules: Option<PathBuf>,
  /// $PWD/node_modules/atlaspack (recursive search)
  pub node_modules_atlaspack: Option<PathBuf>,
  /// $PWD/node_modules/.apvm
  pub node_modules_apvm: Option<PathBuf>,
  /// Local path to Atlaspack source code
  pub atlaspack_local: Option<PathBuf>,
}

impl Paths {
  pub fn new(env: &Env) -> anyhow::Result<Self> {
    let apvm_dir = env.apvm_dir.clone();
    let apvm_global_dir = apvm_dir.join("global");
    let apvm_dir_temp = apvm_dir.join(".temp");
    let apvm_versions_v1_dir = apvm_dir.join("versions_v1");

    let node_modules = find_ancestor_file(&env.pwd, PathBuf::from_iter(&["node_modules"]))?
      .first()
      .cloned();

    let node_modules_apvm =
      find_ancestor_file(&env.pwd, PathBuf::from_iter(&["node_modules", ".apvm"]))?
        .first()
        .cloned();

    let node_modules_atlaspack =
      find_ancestor_file(&env.pwd, PathBuf::from_iter(&["node_modules", "atlaspack"]))?
        .first()
        .cloned();

    if !fs::exists(&apvm_dir)? {
      fs::create_dir(&apvm_dir)?;
    }

    if !fs::exists(&apvm_versions_v1_dir)? {
      fs::create_dir(&apvm_versions_v1_dir)?;
    }

    Ok(Self {
      apvm_dir,
      global_atlaspack: apvm_global_dir.join("atlaspack"),
      global_version: apvm_global_dir.join("version"),
      global: apvm_global_dir,
      temp: apvm_dir_temp,
      versions_v1: apvm_versions_v1_dir,
      node_modules,
      node_modules_apvm,
      node_modules_atlaspack,
      atlaspack_local: env.apvm_atlaspack_local.clone(),
    })
  }

  pub fn temp_dir(&self) -> anyhow::Result<TempDir> {
    if !fs::exists(&self.temp)? {
      fs::create_dir(&self.temp)?;
    }
    TempDir::new(&self.temp)
  }
}
