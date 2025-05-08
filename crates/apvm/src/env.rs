use std::path::PathBuf;

use crate::platform::path_ext::*;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Env {
  pub pwd: PathBuf,
  pub exe_path: PathBuf,
  pub exe_stem: String,
  pub argv: Vec<String>,
  pub apvm_dir: PathBuf,
  pub runtime: String,
}

impl Env {
  pub fn parse() -> anyhow::Result<Self> {
    let pwd = std::env::current_dir()?;

    let mut argv = std::env::args().collect::<Vec<String>>();
    argv.remove(0);

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
