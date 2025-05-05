// TODO validate checksum of tarball

use std::fs;

use flate2::read::GzDecoder;
use log::info;
use log::Level;
use serde::Deserialize;
use tar::Archive;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::exec::exec_blocking;
use crate::platform::exec::ExecOptions;
use crate::platform::package::PackageDescriptor;
use crate::platform::path_ext::PathExt;
use crate::platform::runtime::resolve_runtime;
use crate::platform::temp_dir::TempDir;

#[derive(Debug, Deserialize)]
struct NpmApiResponse {
  dist: NpmApiResponseDist,
}

#[derive(Debug, Deserialize)]
struct NpmApiResponseDist {
  tarball: String,
}

pub fn install_from_npm(
  ctx: Context,
  _cmd: InstallCommand,
  package: PackageDescriptor,
) -> anyhow::Result<()> {
  let target_temp = TempDir::new(&ctx.paths.temp)?;
  let target = ctx.paths.versions_npm.join(&package.version_encoded);

  let url = format!("{}/{}", c::NPM_API_URL, package.version,);

  info!("{}", url);

  println!("Resolving");
  let response = reqwest::blocking::get(&url)?;
  if response.status() != 200 {
    return Err(anyhow::anyhow!(
      "Unable to resolve version {}",
      package.version
    ));
  }

  let bytes = response.bytes()?.to_vec();
  let data = serde_json::from_slice::<NpmApiResponse>(&bytes)?;
  let tarball_url = data.dist.tarball;

  println!("Downloading");
  let response = reqwest::blocking::get(&tarball_url)?;
  if response.status() != 200 {
    return Err(anyhow::anyhow!(
      "Unable to download version {}",
      package.version
    ));
  }

  let bytes = response.bytes()?.to_vec();

  println!("Extracting");
  let tar = GzDecoder::new(bytes.as_slice());
  let mut archive = Archive::new(tar);

  archive.unpack(&target_temp)?;

  let Some(Ok(inner_temp)) = fs::read_dir(&target_temp)?.next() else {
    return Err(anyhow::anyhow!("Unable to find inner package"));
  };

  let mut command_options = ExecOptions {
    cwd: Some(inner_temp.path()),
    silent: true,
    ..Default::default()
  };

  if log::log_enabled!(target: "Global", Level::Info) {
    command_options.silent = false;
  };

  // Packages to post-install (todo get rid of these)
  #[rustfmt::skip]
  exec_blocking(
    [
      resolve_runtime("npm")?.try_to_string()?.as_str(),
      "install",
      "lmdb",
    ],
    command_options,
  )?;

  fs::write(
    inner_temp.path().join(c::APVM_VERSION_FILE),
    package.version.to_string().as_bytes(),
  )?;

  fs::rename(inner_temp.path(), &target)?;

  Ok(())
}
