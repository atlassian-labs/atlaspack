use std::path::PathBuf;

use serde::Serialize;

use crate::platform::constants as c;
use crate::platform::path_ext::*;

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub struct Env {
  pub pwd: PathBuf,
  pub exe_path: PathBuf,
  pub exe_stem: String,
  pub argv: Vec<String>,
  pub apvm_dir: PathBuf,
  pub runtime: String,
  pub atlaspack_npm_url: String,
  pub atlaspack_github_url: String,
  pub atlaspack_release_url: String,
  pub apvm_atlaspack_local: Option<PathBuf>,
}

impl Env {
  pub fn parse() -> anyhow::Result<Self> {
    let pwd = std::env::current_dir()?;

    let argv = std::env::args().collect::<Vec<String>>();
    let exe_path = std::env::current_exe()?;
    let exe_stem = exe_path.try_file_stem()?;

    Ok(Self {
      pwd,
      exe_path,
      exe_stem,
      argv,
      apvm_dir: match std::env::var("APVM_DIR") {
        Ok(apvm_dir) => PathBuf::from(apvm_dir),
        Err(_) => apvm_dir_default()?,
      },
      runtime: match std::env::var("APVM_RUNTIME") {
        Ok(runtime) => runtime,
        Err(_) => "node".to_string(),
      },
      apvm_atlaspack_local: match std::env::var("APVM_ATLASPACK_LOCAL") {
        Ok(apvm_atlaspack_local) => Some(PathBuf::from(apvm_atlaspack_local)),
        Err(_) => None,
      },
      atlaspack_npm_url: match std::env::var("APVM_NPM_URL") {
        Ok(atlaspack_npm_url) => atlaspack_npm_url,
        Err(_) => c::NPM_API_URL.to_string(),
      },
      atlaspack_release_url: match std::env::var("APVM_RELEASE_URL") {
        Ok(atlaspack_release_url) => atlaspack_release_url,
        Err(_) => c::RELEASE_URL.to_string(),
      },
      atlaspack_github_url: match std::env::var("APVM_GITHUB_URL") {
        Ok(atlaspack_github_url) => atlaspack_github_url,
        Err(_) => c::GITHUB_URL.to_string(),
      },
    })
  }
}

fn apvm_dir_default() -> anyhow::Result<PathBuf> {
  let Ok(Some(current_exe)) = homedir::my_home() else {
    return Err(anyhow::anyhow!(
      "Cannot find apvm_home. Please set $APVM_HOME variable manually"
    ));
  };
  let default_dir = current_exe.join(".local").join("apvm");
  if default_dir.is_file() {
    return Err(anyhow::anyhow!("{:?} exists but is a file", current_exe));
  }
  if !default_dir.exists() {
    std::fs::create_dir_all(&default_dir)?;
  }
  Ok(default_dir)
}
