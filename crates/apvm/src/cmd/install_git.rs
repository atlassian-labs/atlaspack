use std::fs;

use flate2::read::GzDecoder;
use log::info;
use log::Level;
use tar::Archive;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::exec::exec_blocking;
use crate::platform::exec::ExecOptions;
use crate::platform::package::GitPackage;
use crate::platform::temp_dir::TempDir;
use crate::public::json_serde::JsonSerde;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

pub fn install_from_git(ctx: Context, cmd: InstallCommand, branch: &str) -> anyhow::Result<()> {
  let pkg = GitPackage::from_name(&ctx.paths.versions_v1, branch)?;
  let temp_dir = TempDir::new(&ctx.paths.temp)?;

  let url = format!(
    "https://github.com/atlassian-labs/atlaspack/archive/{}.tar.gz",
    &branch
  );

  info!("Fetching {}", &url);
  let response = reqwest::blocking::get(&url)?;
  if response.status() == 404 {
    return Err(anyhow::anyhow!("Branch '{}' not found", branch));
  }

  println!("Downloading");
  let bytes = response.bytes()?.to_vec();

  println!("Extracting");
  let tar = GzDecoder::new(bytes.as_slice());
  let mut archive = Archive::new(tar);

  archive.unpack(&temp_dir)?;

  let Some(Ok(inner_temp)) = fs::read_dir(&temp_dir)?.next() else {
    return Err(anyhow::anyhow!("Unable to find inner package"));
  };

  let mut command_options = ExecOptions {
    cwd: Some(inner_temp.path()),
    ..Default::default()
  };

  if log::log_enabled!(target: "Global", Level::Debug) {
    command_options.silent = false;
  };

  if cmd.skip_build {
    if !fs::exists(&pkg.path)? {
      fs::create_dir(&pkg.path)?;
    }
    fs::rename(inner_temp.path(), pkg.contents())?;
    println!("Skipping build steps");
    return Ok(());
  }

  println!("Initializing");
  // Atlaspack needs a .git folder or the build will fail
  fs::create_dir_all(inner_temp.path().join(".git"))?;

  println!("Installing (yarn)");
  exec_blocking(["yarn", "install"], command_options.clone())?;

  println!("Building (Native)");
  exec_blocking(["yarn", "build-native-release"], command_options.clone())?;

  println!("Building (Flow)");
  exec_blocking(["yarn", "build"], command_options.clone())?;

  println!("Building (TypeScript)");
  exec_blocking(["yarn", "build-ts"], command_options.clone())?;

  // Save a little space
  fs::remove_dir_all(inner_temp.path().join("target"))?;

  let package_meta = PackageMeta {
    kind: PackageKind::Git,
    specifier: Some(branch.to_string()),
  };

  // Commit files to versions
  if !fs::exists(&pkg.path)? {
    fs::create_dir(&pkg.path)?;
  }

  package_meta.write_to_file(pkg.meta())?;
  fs::rename(inner_temp.path(), pkg.contents())?;

  Ok(())
}
