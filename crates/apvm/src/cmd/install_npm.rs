// TODO validate checksum of tarball

use std::fs;

use flate2::read::GzDecoder;
use log::info;
use serde::Deserialize;
use tar::Archive;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::package::PackageDescriptor;
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

  let url = format!("{}/{}", c::NPM_API_URL, package.version);

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

  fs::rename(inner_temp.path(), &target)?;

  Ok(())
}
