use std::fs;

use flate2::read::GzDecoder;
use serde::Deserialize;
use tar::Archive;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::exec::exec_blocking;
use crate::platform::exec::ExecOptions;
use crate::platform::fs_ext;
use crate::platform::hash::Integrity;
use crate::platform::http;
use crate::platform::package::NpmPackage;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

#[derive(Debug, Deserialize)]
struct NpmApiResponse {
  dist: NpmApiResponseDist,
}

#[derive(Debug, Deserialize)]
struct NpmApiResponseDist {
  tarball: String,
  integrity: String,
}

pub fn install_from_npm(
  ctx: Context,
  cmd: InstallCommand,
  version: &Specifier,
) -> anyhow::Result<()> {
  let pkg = NpmPackage::from_name(&ctx.paths.versions_v1, version)?;

  let url = format!("{}/{}", c::NPM_API_URL, version.version());

  println!("Resolving");
  log::info!("{}", url);
  let api_response = http::download_serde::<NpmApiResponse>(&url)?;
  let tarball_url = api_response.dist.tarball;
  let integrity = api_response.dist.integrity;

  println!("Downloading");
  let bytes = http::download_bytes(&tarball_url)?;

  if !cmd.skip_checksum && !Integrity::parse(&integrity)?.eq(&bytes) {
    return Err(anyhow::anyhow!("Integrity check failed"));
  }

  println!("Extracting");
  let target_temp = ctx.paths.temp_dir()?;

  let tar = GzDecoder::new(bytes.as_slice());
  let mut archive = Archive::new(tar);
  archive.unpack(&target_temp)?;

  let Some(Ok(inner_temp)) = fs::read_dir(&target_temp)?.next() else {
    return Err(anyhow::anyhow!("Unable to find inner package"));
  };

  if !cmd.skip_postinstall {
    println!("Installing additional dependencies");
    let command_options = ExecOptions {
      cwd: Some(inner_temp.path()),
      ..Default::default()
    };

    exec_blocking(["npm", "install", "lmdb"], &command_options)?;
  }

  println!("Finalizing");

  fs_ext::create_dir_if_not_exists(&pkg.path)?;
  fs_ext::create_dir_if_not_exists(pkg.contents())?;

  PackageMeta::write_to_file(
    &PackageMeta {
      kind: PackageKind::Npm,
      version: Some(version.version().to_string()),
      specifier: Some(version.to_string()),
      checksum: Some(integrity),
    },
    pkg.meta_file(),
  )?;

  fs::rename(inner_temp.path(), pkg.contents())?;
  Ok(())
}
